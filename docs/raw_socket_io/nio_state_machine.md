# NIO 状态机：手撕非阻塞 Socket 编程的核心

## 设计背景

### 为什么 NIO 编程这么难？

在 BIO（Blocking I/O）中，一切都很简单：

```
read() → 阻塞等到数据到达 → 返回 → 处理 → 循环
```

在 NIO（Non-blocking I/O）中，一切都变了：

```
read() → 可能返回 EAGAIN（没数据）→ 返回部分数据 → 需要记住读了多少 →
write() → 可能只写了一半 → 需要记住还剩多少 → 下次继续写 →
accept() → 可能一次来了多个连接 → 需要循环 accept 到 EAGAIN →
```

**NIO 的本质是状态机**。每个连接不再是一个简单的阻塞调用栈，而是一个需要手动管理的状态机。

### Java NIO 怎么做的？

Java 在 JDK 1.4 引入了 NIO，核心 API：

```java
Selector selector = Selector.open();                    // epoll_create
ServerSocketChannel server = ServerSocketChannel.open(); // socket
server.bind(new InetSocketAddress(8080));
server.register(selector, SelectionKey.OP_ACCEPT);      // epoll_ctl ADD

while (true) {
    selector.select();                                   // epoll_wait
    for (SelectionKey key : selector.selectedKeys()) {
        if (key.isAcceptable()) { /* accept */ }
        if (key.isReadable())   { /* read */ }
        if (key.isWritable())   { /* write */ }
    }
}
```

**Java NIO 的问题**：
1. **SelectionKey 是 attachment 模式**——你往 key 上挂任意 Object，类型不安全
2. **Buffer flip/clear 容易搞错**——`buffer.flip()` 忘了调用就读不到数据
3. **没人直接用 Java NIO**——太底层了，Netty 才是实际选择
4. **你无法控制 epoll 行为**——JVM 帮你管理了一切

### Rust NIO 的优势

```rust
// Rust：连接状态是强类型的 enum
enum ConnState {
    Reading { buf: Vec<u8> },
    Writing { buf: Vec<u8>, written: usize },
    HalfClosed,
}
```

Java 的 `SelectionKey.attachment()` 返回 `Object`，你需要 `instanceof` 检查和强制转换。Rust 的 `enum` 让状态转换在编译期就受控——这就是**类型状态模式（Typestate Pattern）**在网络编程中的应用。

---

## 连接状态机

### 状态定义

一个非阻塞 TCP 连接的完整生命周期：

```
                    accept()
                       │
                       ▼
              ┌─────────────────┐
              │   Connected      │
              │  (等待可读/可写)   │
              └────────┬────────┘
                       │
          ┌────────────┼────────────┐
          │ 可读        │            │ 可写
          ▼            │            ▼
   ┌─────────────┐     │     ┌─────────────┐
   │  Reading     │     │     │  Writing     │
   │  读入 buffer │     │     │  写出 buffer │
   └──────┬──────┘     │     └──────┬──────┘
          │            │            │
          │ 读完/       │            │ 写完/
          │ EAGAIN     │            │ EAGAIN
          ▼            │            ▼
   ┌─────────────┐     │     ┌─────────────┐
   │  Waiting     │◄────┘     │  Waiting     │
   │  等待下次事件  │           │  等待下次事件  │
   └──────┬──────┘           └──────┬──────┘
          │                         │
          │ read()=0               │ write() error
          ▼                         ▼
   ┌─────────────┐          ┌─────────────┐
   │ HalfClosed  │          │  Closed      │
   │ 对端关闭     │          │  连接终止     │
   └─────────────┘          └─────────────┘
```

### Rust 实现（类型安全的状态机）

```rust
/// 连接状态——用 enum 强制类型安全
///
/// Java NIO 等价物：SelectionKey.attachment() 返回 Object
/// 你需要 (State) key.attachment() 强制转换，ClassCastException 是运行时错误
/// Rust 的 enum match 是编译期穷尽检查——你不可能遗漏一个状态
enum ConnState {
    /// 正在读取数据到缓冲区
    Reading {
        buf: Vec<u8>,
    },
    /// 有待发送的数据
    Writing {
        buf: Vec<u8>,
        written: usize,  // 已写出多少字节
    },
    /// 对端关闭了写端（收到 FIN）
    HalfClosed,
}

struct Connection {
    stream: TcpStream,
    state: ConnState,
    token: Token,
}

impl Connection {
    fn handle_event(&mut self, event: &Event) -> Action {
        match &mut self.state {
            ConnState::Reading { buf } if event.is_readable() => {
                self.do_read(buf)
            }
            ConnState::Writing { buf, written } if event.is_writable() => {
                self.do_write(buf, written)
            }
            ConnState::HalfClosed => {
                Action::Close  // 半关闭状态，清理连接
            }
            _ => Action::None,
        }
    }
}
```

**与 Java NIO 的对比**：

| 维度 | Rust (enum) | Java NIO (attachment) |
|------|-------------|----------------------|
| 状态定义 | `enum ConnState { ... }` | `class State { ... }` 挂在 `SelectionKey` |
| 类型安全 | 编译期保证 | 运行时 `ClassCastException` |
| 状态遗漏 | `match` 穷尽检查，遗漏编译报错 | `if/else` 链，遗漏无警告 |
| 状态转换 | `self.state = ConnState::Writing { ... }` | `key.attach(newState)` |
| buffer 管理 | `Vec<u8>` 自动扩缩 | `ByteBuffer.flip()/clear()/compact()` 易错 |

---

## Partial I/O：NIO 最核心的问题

### 什么是 Partial I/O？

在非阻塞模式下：
- `read()` 可能只读了 3 字节，但你期望 1024 字节 → **Partial Read**
- `write()` 可能只写了 5 字节，但你给了 100 字节 → **Partial Write**

这是 NIO 和 BIO 的根本区别。BIO 的 `read()`/`write()` 保证阻塞到完成，NIO 不保证。

### Partial Write 的经典模式

这是 NIO 编程中最常见也最容易出 bug 的地方：

```
你想写 100 字节：
  write() → 返回 30  → 还有 70 字节待写
  write() → 返回 EAGAIN → 内核缓冲区满了
  等待 EPOLLOUT 事件...
  EPOLLOUT 到达 → write() → 返回 50 → 还有 20 字节
  write() → 返回 20 → 全部写完
```

#### C 的实现（手动追踪偏移）

```c
struct conn_state {
    int fd;
    char write_buf[4096];
    int write_total;    // 总共要写多少
    int write_offset;   // 已经写了多少
};

void handle_writable(struct conn_state *conn) {
    int remaining = conn->write_total - conn->write_offset;
    int n = write(conn->fd, conn->write_buf + conn->write_offset, remaining);
    if (n < 0) {
        if (errno == EAGAIN) return;  // 缓冲区满，等下次
        perror("write");
        close(conn->fd);
        return;
    }
    conn->write_offset += n;
    if (conn->write_offset == conn->write_total) {
        // 写完了，取消 EPOLLOUT 关注
        struct epoll_event ev = { .events = EPOLLIN, .data.fd = conn->fd };
        epoll_ctl(epfd, EPOLL_CTL_MOD, conn->fd, &ev);
    }
}
```

**C 的 bug 陷阱**：
- `write_total` 和 `write_offset` 不一致 → 数据错乱
- 忘了取消 `EPOLLOUT` → 事件循环空转（100% CPU）
- 多线程并发修改 `write_offset` → 数据竞争

#### Rust 的实现（安全 + 清晰）

```rust
fn do_write(&mut self) -> Action {
    match &mut self.state {
        ConnState::Writing { buf, written } => {
            let remaining = &buf[*written..];
            match self.stream.write(remaining) {
                Ok(n) => {
                    *written += n;
                    if *written >= buf.len() {
                        // 写完了 → 切换到 Reading 状态
                        // 这里体现了 Rust enum 的优势：
                        // 旧状态的数据被自动 drop，新状态开始
                        self.state = ConnState::Reading { buf: vec![0; 4096] };
                    }
                    Action::Reregister(Interest::READABLE)  // 写完只关注可读
                }
                Err(e) if e.kind() == WouldBlock => {
                    Action::None  // 等下次 EPOLLOUT
                }
                Err(e) => {
                    Action::Close
                }
            }
        }
        _ => Action::None,
    }
}
```

**Rust 的安全保障**：
1. `buf` 和 `written` 绑定在 enum variant 中 → 不可能不一致
2. 状态切换时旧数据自动 drop → 不可能泄漏
3. `TcpStream` 的所有权唯一 → 不可能并发写

#### Java NIO 的实现

```java
// Java NIO 的 partial write
class ConnectionState {
    ByteBuffer writeBuf;
    // ...
}

void handleWritable(SelectionKey key) {
    ConnectionState state = (ConnectionState) key.attachment();
    // ⚠️ 强制转换！类型不安全
    try {
        int n = ((SocketChannel) key.channel()).write(state.writeBuf);
        // ⚠️ 又一次强制转换！
        if (!state.writeBuf.hasRemaining()) {
            // 写完了
            key.interestOps(key.interestOps() & ~SelectionKey.OP_WRITE);
            // ⚠️ 位运算 & ~ 忘了就空转！
        }
    } catch (IOException e) {
        key.cancel();
    }
}
```

**Java NIO 的问题**：
1. `attachment()` 返回 `Object` → 强制转换 → 运行时错误
2. `ByteBuffer.flip()` 忘了调 → 读不到数据（经典 bug）
3. `interestOps` 位运算容易出错
4. 没有编译期状态检查

---

## Edge Triggered vs Level Triggered

这是 epoll 编程中最重要的概念，Java NIO 和 Go netpoller 都**不让你选**。

### Level Triggered (LT, 水平触发) — 默认模式

```
有数据未读？ → 每次 epoll_wait 都通知你
```

- 优点：简单，可以不一次读完
- 缺点：频繁通知，性能稍差
- Java NIO Selector 就是 LT 模式（你无法改）

### Edge Triggered (ET, 边沿触发) — 高性能模式

```
数据从无到有？ → 只通知一次
```

- 优点：通知次数少，性能更好
- 缺点：**必须一次读完所有数据**（循环 read 到 EAGAIN），否则永远不再通知
- Nginx、Redis 都用 ET 模式
- Java NIO **不支持** ET 模式

### ET 模式的事件循环

```rust
// Edge Triggered 模式的核心：循环 read/write 到 EAGAIN
fn handle_readable_et(&mut self) -> Action {
    let mut buf = [0u8; 4096];
    loop {
        match self.stream.read(&mut buf) {
            Ok(0) => return Action::Close,      // 对端关闭
            Ok(n) => {
                // 处理数据...
                self.process_data(&buf[..n]);
            }
            Err(e) if e.kind() == WouldBlock => {
                break;  // EAGAIN → 数据读完了，退出循环
            }
            Err(_) => return Action::Close,
        }
    }
    Action::None
}
```

**对比 Java NIO**：Java 的 Selector 只支持 LT 模式，你不需要（也不能）做这种循环读。这意味着 Java NIO 的吞吐量上限被 LT 模式锁死了。

### 为什么 ET 更快？

```
LT 模式（100 个请求，每次 10 字节）：
  epoll_wait × 100 → 每次都通知 → 100 次系统调用

ET 模式（100 个请求，每次 10 字节）：
  epoll_wait × 1 → 通知一次 → 循环 read 100 次 → 1 次系统调用
```

减少 `epoll_wait` 调用次数 = 减少用户态/内核态切换 = 更高吞吐量。

---

## epoll 事件空转问题

这是 NIO 编程中的经典性能杀手：

### 问题

```rust
// 注册时同时关注 READABLE 和 WRITABLE
poll.registry().register(&mut conn.stream, token,
    Interest::READABLE | Interest::WRITABLE);

// 问题：WRITABLE 在内核缓冲区不满时总是就绪
// → epoll_wait 每次都返回 WRITABLE 事件
// → 你的事件循环被无意义的 WRITABLE 事件淹没
// → CPU 100%
```

### 解决方案：动态调整 Interest

```rust
/// 连接动作——事件处理后返回需要做什么
enum Action {
    None,
    Reregister(Interest),
    Close,
}

// 读取数据后：
//   - 如果有数据要写 → 关注 READABLE | WRITABLE
//   - 如果没有数据要写 → 只关注 READABLE
// 这就是 Netty 的 flush 策略的底层原理

fn handle_read(&mut self) -> Action {
    match self.stream.read(&mut self.read_buf) {
        Ok(n) => {
            // 假设要做 echo，把读到的数据加入写缓冲
            let data = self.read_buf[..n].to_vec();
            self.state = ConnState::Writing { buf: data, written: 0 };
            // 有数据要写 → 同时关注可读可写
            Action::Reregister(Interest::READABLE | Interest::WRITABLE)
        }
        Err(e) if e.kind() == WouldBlock => Action::None,
        _ => Action::Close,
    }
}

fn handle_write(&mut self) -> Action {
    match &mut self.state {
        ConnState::Writing { buf, written } => {
            match self.stream.write(&buf[*written..]) {
                Ok(n) => {
                    *written += n;
                    if *written >= buf.len() {
                        // 写完了 → 只关注可读（避免空转）
                        self.state = ConnState::Reading { buf: vec![0; 4096] };
                        Action::Reregister(Interest::READABLE)
                    } else {
                        Action::None
                    }
                }
                Err(_) => Action::Close,
                _ => Action::None,
            }
        }
        _ => Action::None,
    }
}
```

**这是 NIO 编程最核心的技巧**：只在需要时关注 WRITABLE，写完立即取消。

---

## Buffer 管理：Rust vs Java NIO

### Java NIO 的 ByteBuffer

```java
ByteBuffer buf = ByteBuffer.allocate(1024);

// 读入
int n = channel.read(buf);  // position 前移

// 切换到读模式
buf.flip();   // ⚠️ 忘了调就全是 0

// 处理数据
byte[] data = new byte[buf.remaining()];
buf.get(data);

// 切换回写模式
buf.clear();  // ⚠️ compact() vs clear() 容易搞混
```

**Java 的 flip/clear 是 bug 的温床**：
- `flip()` 忘了调 → 读不到数据
- `compact()` vs `clear()` 搞混 → 数据丢失
- `ByteBuffer` 的 position/limit/capacity 三指针极容易出错

### Rust 的 buffer 管理

```rust
// Rust：简单的 Vec<u8>，不需要 flip/clear
let mut buf = vec![0u8; 1024];

// 读入
let n = stream.read(&mut buf)?;  // 返回读了多少

// 处理数据——直接用切片
let data = &buf[..n];  // 不可能搞错范围

// 写出
stream.write_all(data)?;  // write_all 保证全部写完（阻塞模式）
// 或手动 partial write（非阻塞模式）
```

**为什么 Rust 不需要 flip/clear？**

Java 的 `ByteBuffer` 把"缓冲区容量"和"有效数据长度"混在同一个对象里，需要 `flip()` 在两者间切换。Rust 直接用 `&buf[..n]` 切片，有效数据长度由 `n` 显式给出。

---

## 完整对比表

| 维度 | Rust NIO (mio) | Java NIO | C epoll |
|------|---------------|----------|---------|
| **状态管理** | `enum ConnState` 编译期安全 | `attachment()` 运行时类型转换 | 手写 struct |
| **buffer** | `Vec<u8>` + 切片 | `ByteBuffer.flip()` 易错 | `char[]` + 偏移 |
| **partial I/O** | `drain(..n)` 自动管理 | `compact()` 手动管理 | `memmove` 手动管理 |
| **ET 模式** | ✅ 支持 | ❌ 不支持 | ✅ 支持 |
| **Interest 调整** | `reregister()` | `interestOps()` 位运算 | `epoll_ctl(MOD)` |
| **fd 安全** | `Drop` 自动关闭 | GC 间接保证 | 手动 `close()` |
| **事件循环** | `poll.poll()` | `selector.select()` | `epoll_wait()` |
| **跨平台** | mio 统一 epoll/IOCP/kqueue | JVM 统一 | 手写 `#ifdef` |

---

## 设计哲学总结

### NIO 编程的本质

NIO 不是"非阻塞 read/write"那么简单。它是一种**事件驱动编程模型**：

1. **事件是通知，不是数据**——epoll 告诉你"可读了"，你自己去读
2. **状态机是核心**——每个连接是一个小型状态机
3. **缓冲区是你的责任**——没有人替你管理 partial I/O
4. **性能陷阱到处都是**——事件空转、ET 遗漏、buffer 错误

### Rust 在 NIO 中的独特价值

```
C  的 NIO：    完全控制    +    完全不安全
Java 的 NIO：  部分控制    +    部分安全
Rust 的 NIO：  完全控制    +    编译期安全
```

Rust 让你在获得 C 级别控制力的同时，用类型系统消除了 NIO 编程中最常见的 bug：
- 连接状态用 enum → 不可能遗漏状态
- buffer 用切片 → 不可能 flip/clear 错误
- fd 用 Drop → 不可能泄漏
- Interest 用 enum → 不可能位运算错误

---

## 运行

```bash
# 运行带状态机的 NIO echo server
cargo run -p nio_state_machine
```

**测试**：

```bash
# 终端 1：启动 server
cargo run -p nio_state_machine

# 终端 2：连接测试
nc 127.0.0.1 8080

# 终端 3：并发连接
nc 127.0.0.1 8080
```

**观察日志输出**，你会看到：
- 连接建立时的状态转换（Connected → Reading）
- 数据到达时的状态转换（Reading → Writing → Reading）
- 部分写入时的 Interest 调整
- 连接关闭时的清理

---

## 延伸：从 NIO 到 async/await

```
手写 NIO 状态机    →    mio    →    tokio (async/await)
   (本模块)        (上层模块)     (生产环境)

   你自己管理       mio 统一       tokio 自动
   状态机           epoll/IOCP     管理 all
   buffer           但状态机       编译器重写
   Interest         仍是你管       为状态机
```

理解 NIO 状态机是理解 tokio 内部机制的基础。tokio 的 `async fn` 本质上就是编译器帮你生成的状态机——它做的正是你在本模块中手写的那些事。
