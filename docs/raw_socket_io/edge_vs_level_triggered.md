# Edge Triggered vs Level Triggered：代码层面的全部区别

## 一句话总结

**ET 必须循环到 EAGAIN，LT 可以只读一次。注册差一个 flag，处理差一个 loop。**

---

## 注册时的代码差异

### C — epoll_ctl

```c
// ========== Level Triggered（默认）==========
struct epoll_event ev;
ev.events = EPOLLIN;                    // 没有 EPOLLET
ev.data.fd = fd;
epoll_ctl(epfd, EPOLL_CTL_ADD, fd, &ev);

// ========== Edge Triggered ==========
struct epoll_event ev;
ev.events = EPOLLIN | EPOLLET;          // 多一个 EPOLLET
ev.data.fd = fd;
epoll_ctl(epfd, EPOLL_CTL_ADD, fd, &ev);
```

**就这么一个 flag 的区别。**

### Rust — libc FFI

```rust
// ========== Level Triggered ==========
let mut ev = libc::epoll_event {
    events: libc::EPOLLIN as u32,                    // 没有 EPOLLET
    u: libc::epoll_data { fd },
};

// ========== Edge Triggered ==========
let mut ev = libc::epoll_event {
    events: (libc::EPOLLIN | libc::EPOLLET) as u32,  // 多一个 EPOLLET
    u: libc::epoll_data { fd },
};
```

### mio — 只有 LT

```rust
// mio 只有 Level Triggered，没有 edge trigger 选项
poll.registry().register(&mut stream, token, Interest::READABLE)?;
// mio 在所有平台上都是 LT，这是设计决策
```

---

## 事件处理时的代码差异（核心！）

这才是 ET 和 LT 真正的区别。不是注册，而是**你收到事件后怎么处理**。

### accept 的差异

```
场景：3 个客户端同时连接，epoll_wait 只通知一次
```

#### LT（Level Triggered）

```c
// LT：可以只 accept 一次，没取完的连接下次 epoll_wait 还会通知
// 因为 LT 的语义是："有未处理的连接？每次都通知你"
void handle_accept_lt(int epfd, int listen_fd) {
    struct sockaddr_in addr;
    socklen_t len = sizeof(addr);
    int client_fd = accept(listen_fd, &addr, &len);  // 只取一个
    if (client_fd >= 0) {
        // 注册 client_fd ...
    }
    // 即使还有 2 个连接没取，下次 epoll_wait 还会通知
    // → 不会丢连接，只是慢一点
}
```

```rust
// Rust LT (mio)
fn handle_accept_lt(server: &mut TcpListener) -> io::Result<()> {
    if let Ok((stream, addr)) = server.accept() {  // 只取一个
        println!("Accepted: {}", addr);
        // ...
    }
    Ok(())
}
```

#### ET（Edge Triggered）

```c
// ET：必须循环 accept 直到 EAGAIN，否则永远不会再次通知
// 因为 ET 的语义是："状态从无连接变成有连接？通知一次，之后不管了"
void handle_accept_et(int epfd, int listen_fd) {
    while (1) {                              // ← 循环！
        struct sockaddr_in addr;
        socklen_t len = sizeof(addr);
        int client_fd = accept(listen_fd, &addr, &len);
        if (client_fd < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                break;                       // 所有连接取完了
            }
            perror("accept");
            break;
        }
        // 注册 client_fd ...
    }
}
```

```rust
// Rust ET
fn handle_accept_et(server: &mut TcpListener) -> io::Result<()> {
    loop {                                   // ← 循环！
        match server.accept() {
            Ok((stream, addr)) => {
                println!("Accepted: {}", addr);
                // ...
            }
            Err(e) if e.kind() == WouldBlock => {
                break;                       // EAGAIN → 取完了
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
    Ok(())
}
```

**对比图**：

```
3 个客户端同时连接：

LT:
  epoll_wait → accept(A) → 返回
  epoll_wait → accept(B) → 返回        ← 3 次 epoll_wait
  epoll_wait → accept(C) → 返回

ET:
  epoll_wait → accept(A) → accept(B) → accept(C) → EAGAIN → 返回
                                         ← 1 次 epoll_wait
```

### read 的差异

```
场景：对端发了 10KB 数据，你的 buffer 是 4KB
```

#### LT

```c
// LT：可以只读一次，剩下的数据下次 epoll_wait 还会通知
void handle_read_lt(int epfd, int fd) {
    char buf[4096];
    int n = read(fd, buf, sizeof(buf));     // 读 4KB，还剩 6KB
    // ... 处理这 4KB ...
    // 不用担心！剩下的 6KB 下次 epoll_wait 还会通知你
}
```

#### ET

```c
// ET：必须循环 read 直到 EAGAIN，否则剩余数据永远不会通知
void handle_read_et(int epfd, int fd) {
    char buf[4096];
    while (1) {                             // ← 循环！
        int n = read(fd, buf, sizeof(buf));
        if (n < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                break;                      // 数据读完了
            }
            // 错误，关闭连接
            break;
        }
        if (n == 0) {
            // 对端关闭
            break;
        }
        // ... 处理这 n 字节 ...
    }
}
```

```rust
// Rust ET read
fn handle_read_et(conn: &mut Connection) -> io::Result<()> {
    let mut buf = [0u8; 4096];
    loop {                                  // ← 必须循环！
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                // 对端关闭
                return Ok(());
            }
            Ok(n) => {
                conn.process_data(&buf[..n]);
            }
            Err(e) if e.kind() == WouldBlock => {
                break;                      // EAGAIN → 读完了
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(())
}
```

### write 的差异

```
场景：你想写 10KB，内核缓冲区只有 4KB 空间
```

#### LT

```c
// LT：write 一部分，下次 EPOLLOUT 还会通知
int written = write(fd, data, total);
// written = 4KB, 还剩 6KB
// 下次 epoll_wait 返回 EPOLLOUT → 继续写
```

#### ET

```c
// ET：write 一部分后如果缓冲区没满，继续写
// 如果 write 返回 EAGAIN → 记住剩余量，等下次 EPOLLOUT
void handle_write_et(int fd, char *data, int total) {
    int offset = 0;
    while (offset < total) {                // ← 循环！
        int n = write(fd, data + offset, total - offset);
        if (n < 0) {
            if (errno == EAGAIN) {
                // 缓冲区满了，等 EPOLLOUT
                // 必须记住 offset！下次从 offset 继续
                break;
            }
            // 错误
            break;
        }
        offset += n;
    }
    if (offset < total) {
        // 没写完，需要注册 EPOLLOUT
        // 等下次 EPOLLOUT 通知后继续写
    }
}
```

---

## 完整对比表

| 维度 | Level Triggered | Edge Triggered |
|------|----------------|----------------|
| **注册** | `events = EPOLLIN` | `events = EPOLLIN \| EPOLLET` |
| **accept** | 可以只 accept 一次 | **必须** 循环 accept 到 EAGAIN |
| **read** | 可以只 read 一次 | **必须** 循环 read 到 EAGAIN |
| **write** | 写一部分，等下次 EPOLLOUT | 循环 write 到 EAGAIN 或全部写完 |
| **漏处理后果** | 下次还会通知，安全 | **永远不再通知，丢数据** |
| **epoll_wait 调用次数** | 多（每次都通知） | 少（只在状态变化时通知） |
| **用户态/内核态切换** | 多 | 少 |
| **吞吐量** | 稍低 | 稍高 |
| **编程难度** | 简单 | 难（必须循环到 EAGAIN） |
| **谁在用** | Java NIO, mio | Nginx, Redis, Netty (epoll) |

---

## ET 的隐蔽 bug

### Bug 1：忘了循环 → 数据永久丢失

```c
// 错误！ET 模式下只读一次
void handle_read_et_wrong(int fd) {
    char buf[4096];
    int n = read(fd, buf, sizeof(buf));  // 只读一次
    // 如果对端发了 8KB，你只读了 4KB
    // 剩余 4KB 永远不会被通知 → 连接"假死"
}
```

### Bug 2：accept 不循环 → 连接丢失

```c
// 错误！ET 模式下只 accept 一次
void handle_accept_et_wrong(int listen_fd) {
    int fd = accept(listen_fd, NULL, NULL);  // 只取一个
    // 如果有 5 个连接排队，只取了 1 个
    // 剩余 4 个永远不会被通知 → 客户端连接超时
}
```

### Bug 3：ET + 非阻塞 必须配对

```c
// 错误！ET 模式 + 阻塞 fd = 死锁
int fd = socket(AF_INET, SOCK_STREAM, 0);  // 阻塞模式！
struct epoll_event ev = { .events = EPOLLIN | EPOLLET };
epoll_ctl(epfd, EPOLL_CTL_ADD, fd, &ev);
// 如果循环 read 到 EAGAIN 时，因为 fd 是阻塞的，
// read 会阻塞而不是返回 EAGAIN → 整个事件循环卡死
```

**必须**：ET 模式的 fd **必须**设为非阻塞。

### Bug 4：饥饿问题

```c
// ET 模式下，一个连接上不断有数据到达
// 如果你不限制每次处理的量，这个连接会独占事件循环
while (1) {
    int n = read(fd, buf, sizeof(buf));
    if (n <= 0) break;
    process(buf, n);
    // 如果对端发送速度很快，这个循环永远不会结束
    // 其他连接永远得不到处理 → 饥饿
}
```

**解决**：限制每次循环的最大次数

```c
#define MAX_READS_PER_EVENT 64

int count = 0;
while (count++ < MAX_READS_PER_EVENT) {
    int n = read(fd, buf, sizeof(buf));
    if (n <= 0) break;
    process(buf, n);
}
// 即使没读完，也让出 CPU 给其他连接
// 下次 EPOLLIN（对端再发新数据时）继续
```

---

## Rust 完整实现：LT vs ET

以下是完整的对比代码，使用 mio (LT) 和 raw epoll (ET)：

### LT 版本（mio，跨平台）

```rust
// LT：每次事件只处理一次 read/write
fn handle_readable_lt(conn: &mut Connection) -> Action {
    let mut buf = [0u8; 4096];
    match conn.stream.read(&mut buf) {
        Ok(0) => Action::Close,
        Ok(n) => {
            conn.write_buf.extend_from_slice(&buf[..n]);
            Action::Reregister(Interest::READABLE | Interest::WRITABLE)
        }
        Err(_) => Action::None,
    }
    // 不需要循环！没读完的数据，下次 poll.poll() 还会通知
}
```

### ET 版本（raw epoll，Linux only）

```rust
// ET：必须循环 read 到 EAGAIN
fn handle_readable_et(conn: &mut Connection) -> Action {
    let mut total_read = 0;
    let mut buf = [0u8; 4096];
    let max_reads = 64; // 防饥饿限制

    for _ in 0..max_reads {
        match conn.stream.read(&mut buf) {
            Ok(0) => return Action::Close,
            Ok(n) => {
                total_read += n;
                conn.write_buf.extend_from_slice(&buf[..n]);
            }
            Err(e) if e.kind() == WouldBlock => {
                // EAGAIN → 所有数据读完了
                break;
            }
            Err(_) => return Action::Close,
        }
    }

    if total_read > 0 {
        // 有数据要 echo 回去
        Action::Reregister(Interest::READABLE | Interest::WRITABLE)
    } else {
        Action::None
    }
}
```

**这就是全部的代码差异。**

---

## 为什么 Nginx/Redis 选 ET，Java/Go 选 LT？

| 选择 | 原因 |
|------|------|
| **Nginx (ET)** | 单线程事件循环，减少 epoll_wait 调用 = 更高吞吐 |
| **Redis (ET)** | 同上，单线程 + ET = 最少的系统调用 |
| **Java NIO (LT)** | JVM 需要跨平台，LT 的语义更容易统一 |
| **Go netpoller (LT)** | runtime 管理，不需要暴露给用户 |
| **mio (LT)** | 跨平台统一 API，LT 在所有平台上都可行 |
| **Netty epoll (ET)** | Netty 有 Linux 优化的 ET 模式（`EpollEventLoopGroup`） |

**关键洞察**：ET 的性能优势来自减少 `epoll_wait` 系统调用次数。在高并发场景下（万级连接），这个差异可能达到 10-30%。但在大多数业务场景中，LT 的性能已经足够。

---

## 运行

```bash
# 运行 LT vs ET 对比 demo
cargo run -p et_vs_lt
```

程序会启动一个 LT echo server，同时在注释中展示 ET 模式的等价代码。

---

## 给 Java 程序员的一句话

Java 的 `Selector` 只有 LT 模式。你在 Java NIO 中写的 `channel.read(buf)` 只需要读一次——这是因为 JVM 帮你做了 LT。如果 Java 支持 ET，你的代码就必须像上面的 ET 版本一样循环到 EAGAIN，否则数据就丢了。

**这就是为什么 Java NIO 简单但性能有天花板，而 C/Rust 的 ET 模式难写但吞吐更高。**
