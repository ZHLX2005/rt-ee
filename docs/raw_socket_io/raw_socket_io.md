# Raw Socket I/O：Rust 能否像 C 一样手撕 socket？

## 设计背景与问题域

### 核心问题

C 语言程序员对 socket 编程的掌控感来自三件事：

1. **直接操作 fd**：`socket()` 返回的就是一个整数文件描述符
2. **手动管理 I/O 多路复用**：`epoll_ctl` / `epoll_wait` 精确控制哪些 fd 可读/可写
3. **完全控制 socket 状态机**：`bind` → `listen` → `accept` → `read/write` → `close`，每一步你都清楚发生了什么

Rust 能做到吗？**答案是：完全能，而且能做得更好。**

但"更好"不是指更快（底层都是同一组系统调用），而是指：

- **同样的底层控制力**：Rust 可以直接调用 `epoll_create`、`epoll_ctl`、`epoll_wait` 等系统调用
- **额外的安全保障**：借用检查器防止了 C 中最常见的 socket 编程 bug（use-after-free、double close、数据竞争）
- **零成本抽象**：高层抽象（mio/tokio）编译后的代码和你手写的 C epoll 代码性能等价

### 为什么这个问题很重要？

对 Java/Go 程序员来说，这个问题的本质是：

> **我能控制到什么粒度？**

| 语言 | 最底层控制粒度 | 能否直接调用 epoll？ | 典型抽象层 |
|------|--------------|-------------------|-----------|
| **C** | 系统调用 | ✅ 直接 syscall | 无（裸调用） |
| **Rust** | 系统调用 | ✅ 通过 libc FFI | libc → mio → tokio |
| **Go** | runtime 调度器 | ❌ 被 netpoller 封装 | net package（内部用 epoll/IOCP） |
| **Java** | JVM NIO | ❌ 被 Selector 封装 | NIO/Netty（内部用 epoll/kqueue） |

**关键洞察**：Rust 是唯一同时具备"系统级控制力"和"高级语言安全性"的语言。

- Go 的 `net.Listen` 内部虽然用了 epoll，但你无法直接控制它——Go runtime 的 netpoller 替你管理了一切
- Java 的 NIO Selector 封装了 epoll，但 JVM 是一个厚重的抽象层，你无法直接调用 `epoll_ctl`
- Rust 的 `libc::epoll_create1(...)` 是对 syscall 的**直接绑定**，没有中间层

---

## Rust 网络编程的四层模型

Rust 的网络 I/O 可以在四个层次上工作，每一层都可以选择：

```
┌─────────────────────────────────────────┐
│  Layer 4: tokio (异步运行时)               │  async/await + 自动调度
├─────────────────────────────────────────┤
│  Layer 3: mio (事件循环)                  │  epoll/IOCP/kqueue 统一抽象
├─────────────────────────────────────────┤
│  Layer 2: socket2 + libc (系统调用)       │  直接调用 OS API
├─────────────────────────────────────────┤
│  Layer 1: 内联汇编 / raw syscall          │  绕过 libc，直接 syscall
└─────────────────────────────────────────┘
```

### Layer 1：内联汇编直接 syscall（极少使用）

```rust
// 极端情况：绕过 libc，直接用汇编发起系统调用
// Linux x86_64 的 epoll_create1 syscall number = 291
unsafe {
    let epfd: i32;
    core::arch::asm!(
        "syscall",
        in("rax") 291u64,          // epoll_create1
        in("rdi") 0u64,            // flags = 0
        lateout("rax") epfd,
        out("rcx") _,
        out("r11") _,
    );
    // epfd 就是 epoll 实例的 fd
}
```

**为什么不推荐**：libc 已经是 syscall 的薄封装（一个 `syscall` 指令），没有性能差异。只有在不能用 libc 的特殊场景（如 OS 内核开发）才需要。

### Layer 2：socket2 + libc（手动控制，最像 C）

这是最接近 C socket 编程的方式，也是理解 Rust 网络编程底层机制的最佳入口。

```rust
use std::net::SocketAddr;
use socket2::{Socket, Domain, Type, Protocol};

// 创建 socket —— 和 C 的 socket(AF_INET, SOCK_STREAM, 0) 等价
let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();

// 设置 SO_REUSEADDR —— 和 C 的 setsockopt 等价
socket.set_reuse_address(true).unwrap();

// 绑定地址 —— 和 C 的 bind() 等价
let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
socket.bind(&addr.into()).unwrap();

// 监听 —— 和 C 的 listen() 等价
socket.listen(128).unwrap();

// 关键区别：Rust 的 Socket 在 drop 时自动 close fd
// C 需要手动 close(fd)，忘了就泄漏
```

**设计对比**：

| 维度 | C | Rust |
|------|---|------|
| socket 创建 | `int fd = socket(...)` | `Socket::new(...)` 返回拥有 fd 的类型 |
| close 时机 | 手动 `close(fd)` | `Drop` trait 自动关闭（RAII） |
| fd 泄漏风险 | 高（忘记 close） | 极低（编译器保证） |
| 错误处理 | 检查返回值 == -1 | Result 类型，强制处理 |

**直接调用 epoll（Linux）**：

```rust
use libc::{self, epoll_create1, epoll_ctl, epoll_wait, EPOLLIN, EPOLLOUT};
use libc::{EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD};

fn raw_epoll_example() {
    unsafe {
        // 1. 创建 epoll 实例 —— 等价于 C: int epfd = epoll_create1(0);
        let epfd = epoll_create1(0);
        if epfd == -1 {
            panic!("epoll_create1 failed: {}", std::io::Error::last_os_error());
        }

        // 2. 创建监听 socket
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_NONBLOCK, 0);

        // 3. 注册到 epoll —— 等价于 C: epoll_ctl(epfd, EPOLL_CTL_ADD, fd, &ev);
        let mut ev = libc::epoll_event {
            events: EPOLLIN as u32,
            u64: listen_fd as u64,
        };
        epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &mut ev);

        // 4. 事件循环 —— 等价于 C: epoll_wait(epfd, events, max_events, timeout);
        let mut events = vec![libc::epoll_event { events: 0, u64: 0 }; 1024];
        loop {
            let n = epoll_wait(epfd, events.as_mut_ptr(), 1024, -1);
            for i in 0..n as usize {
                let fd = events[i].u64 as i32;
                let evts = events[i].events;
                if evts & (EPOLLIN as u32) != 0 {
                    // 可读事件：accept 或 read
                    if fd == listen_fd {
                        let client_fd = libc::accept4(listen_fd, std::ptr::null_mut(), std::ptr::null_mut(), libc::SOCK_NONBLOCK);
                        // 将 client_fd 注册到 epoll...
                    } else {
                        // 从 client_fd 读取数据...
                    }
                }
                if evts & (EPOLLOUT as u32) != 0 {
                    // 可写事件
                }
            }
        }
    }
}
```

**这段代码和 C 的 epoll 代码几乎一模一样**——唯一的区别是它被包在 `unsafe {}` 块中。

为什么需要 `unsafe`？因为：
- `epoll_wait` 返回的 fd 可能在另一个线程中被关闭 → 垂悬指针
- `accept4` 可能返回 -1（错误），你需要检查
- 并发修改 epoll 实例可能导致未定义行为

Rust 的 `unsafe` 不是"不允许"，而是说："这里编译器无法为你提供安全保证，你需要自己确保正确性。"

### Layer 3：mio（跨平台事件循环）

mio 是 tokio 的底层，它在不同平台上选择最优的 I/O 多路复用机制：

| 平台 | 底层实现 |
|------|---------|
| Linux | epoll |
| macOS | kqueue |
| Windows | IOCP (I/O Completion Ports) |
| FreeBSD | kqueue |

```rust
use mio::{Events, Interest, Poll, Token};
use mio::net::TcpListener;

const SERVER: Token = Token(0);

fn mio_example() -> std::io::Result<()> {
    // 创建 poll 实例 —— 在 Linux 上就是 epoll_create1()
    let mut poll = Poll::new()?;  // ← 内部调用 epoll_create1 / CreateIoCompletionPort

    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;

    // 注册感兴趣的事件 —— 内部调用 epoll_ctl / CreateIoCompletionPort
    poll.registry().register(
        &mut server,
        SERVER,
        Interest::READABLE,  // ← 内部设置 EPOLLIN
    )?;

    let mut events = Events::with_capacity(1024);

    loop {
        // 等待事件 —— 内部调用 epoll_wait / GetQueuedCompletionStatus
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                SERVER => {
                    // 可读 = 有新连接
                    let (mut conn, addr) = server.accept()?;
                    println!("Accepted: {}", addr);

                    // 注册新连接
                    let token = Token(event.token().0 + 1);
                    poll.registry().register(
                        &mut conn,
                        token,
                        Interest::READABLE | Interest::WRITABLE,
                    )?;
                }
                Token(id) => {
                    println!("Event on connection {}", id);
                    // 处理数据...
                }
            }
        }
    }
}
```

**设计哲学**：

mio 的设计目标不是"隐藏 epoll"，而是"统一 epoll/kqueue/IOCP 的接口"。它是一个**薄抽象**——

- 你仍然知道底层在用 epoll
- 你仍然控制 register/deregister 的时机
- 你仍然自己管理 buffer 和读写逻辑
- 抽象成本几乎为零（编译后和手写 epoll 代码等价）

### Layer 4：tokio（异步运行时）

tokio 在 mio 之上提供了 async/await 语法糖：

```rust
use tokio::net::TcpListener;

async fn tokio_example() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Accepted: {}", addr);

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let n = socket.read(&mut buf).await?;
                if n == 0 { return Ok(()); }
                socket.write_all(&buf[..n]).await?;
            }
        });
    }
}
```

**关键区别**：tokio 帮你管理了"当 fd 不可读时挂起，可读时恢复"的调度逻辑。底层仍然是 mio → epoll/IOCP。

---

## Windows 上的等价物：IOCP

Rust 在 Windows 上用 IOCP（I/O Completion Ports）替代 epoll。这是 Windows 上最高性能的 I/O 模型。

### IOCP vs epoll 的本质区别

这是一个极其重要的架构差异，Java NIO 和 Go netpoller 在 Windows 上也都用 IOCP：

| 维度 | epoll (Linux) | IOCP (Windows) |
|------|--------------|----------------|
| 编程模型 | **边缘触发 / 水平触发** | **完成端口**（完成驱动） |
| 通知时机 | fd 就绪（可以 read/write） | I/O 操作已完成（数据已到达 buffer） |
| buffer 管理 | 你自己提供，epoll 不碰 | 你提供 buffer，OS 填入数据后通知你 |
| 调用方式 | `epoll_wait` → 检查事件 → 自己 `read` | `ReadFile` 发起请求 → `GetQueuedCompletionStatus` 收到完成通知 |
| 线程模型 | 单线程事件循环 | 多线程从完成端口取结果 |

**这个区别是根本性的**：

- epoll 告诉你 "fd 可读了"，你自己调用 `read` → 这是**就绪驱动**
- IOCP 告诉你 "你之前发起的 read 操作完成了" → 这是**完成驱动**

这就是为什么 Java NIO 在 Windows 上的性能不如 Linux——JVM 的 Selector 接口是就绪驱动模型，而 IOCP 是完成驱动模型，JVM 不得不做一层不太高效的适配。

mio 在 Windows 上做了一层适配，让你用类似 epoll 的就绪驱动 API 来使用 IOCP。

---

## Rust socket 编程的安全保障

这是 Rust 相比 C 的核心优势。以下 bug 在 C 中常见，在 Rust 中被编译器阻止：

### 1. Use-After-Free（使用已关闭的 fd）

```c
// C: 编译通过，运行时产生未定义行为
int fd = socket(AF_INET, SOCK_STREAM, 0);
close(fd);
write(fd, data, len);  // UB! fd 已被关闭，可能被复用
```

```rust
// Rust: 编译失败
let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
drop(socket);  // fd 被 close
socket.write(buf);  // 编译错误！socket 的所有权已被 move
```

### 2. 数据竞争（多线程同时操作同一个 fd）

```c
// C: 编译通过，运行时数据竞争
int fd = socket(AF_INET, SOCK_STREAM, 0);
// 线程 A 和线程 B 同时 write(fd, ...) → 数据竞争
```

```rust
// Rust: 编译失败（Socket 不实现 Send，或需要 Arc<Mutex<Socket>>）
let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
let socket2 = socket;  // move，socket 不再可用
// 如果要跨线程共享，必须显式 Arc<Mutex<Socket>>，编译器强制你同步
```

### 3. fd 泄漏（忘记 close）

```c
// C: 编译通过，fd 泄漏
int fd = socket(AF_INET, SOCK_STREAM, 0);
if (error_condition) {
    return -1;  // 忘记 close(fd)，fd 泄漏
}
close(fd);
```

```rust
// Rust: Socket 的 Drop 自动 close fd
fn example() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    if error_condition {
        return Err(...);  // socket 自动 drop → fd 自动 close
    }
    // 正常流程结束也会自动 drop
}
```

---

## 设计决策对比表

| 维度 | Rust | C | Java | Go |
|------|------|---|------|-----|
| **直接 syscall** | ✅ libc FFI | ✅ 直接调用 | ❌ JNI 才行 | ❌ 被 runtime 封装 |
| **epoll 访问** | ✅ 直接 `epoll_create1` | ✅ 直接调用 | ❌ Selector 封装 | ❌ netpoller 封装 |
| **IOCP 访问** | ✅ windows-sys | ✅ 直接调用 | ❌ Selector 封装 | ❌ netpoller 封装 |
| **零拷贝 I/O** | ✅ `splice`/`sendfile` | ✅ 直接调用 | ❌ JVM 限制 | ⚠️ 有限支持 |
| **fd 安全** | 编译期保证 | 无保证 | GC 间接保证 | GC 间接保证 |
| **内存安全** | 编译期保证 | 无保证 | GC 保证 | GC 保证 |
| **抽象成本** | 零成本（编译后等价 C） | 无抽象 | JVM 开销 | runtime 开销 |
| **跨平台** | mio 统一 | 需手写 #ifdef | JVM 统一 | runtime 统一 |

---

## 实际工程建议

### 什么时候手撕 epoll/IOCP？

| 场景 | 建议 | 原因 |
|------|------|------|
| 学习网络原理 | ✅ 手撕 | 理解 epoll/IOCP 工作原理 |
| 自定义协议栈 | ✅ 手撕或用 mio | 需要精确控制 fd 状态 |
| 高性能代理/网关 | ⚠️ 用 mio | 性能接近手撕，代码更安全 |
| 业务服务 | ❌ 用 tokio | async/await 提高生产力 |
| 简单 TCP/UDP 工具 | ❌ 用 std::net | 标准库足够 |

### Crate 选择指南

```
需要什么？
├── 直接调用 syscall → libc / windows-sys
├── 跨平台事件循环 → mio（tokio 的底层）
├── 异步运行时 → tokio / async-std
├── 简单同步网络 → std::net / socket2
└── 超低延迟 (DPDK/RDMA) → 特殊 crate + FFI
```

---

## 运行

本模块的 lab 代码使用 mio 实现跨平台事件驱动 echo server：

```bash
# 运行 mio 版 echo server
cargo run -p raw_socket_io
```

然后在另一个终端：
```bash
# 连接测试
nc 127.0.0.1 8080
# 或
telnet 127.0.0.1 8080
```
