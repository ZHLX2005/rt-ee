// ============================================================================
// Raw Socket I/O: Rust 手撕事件驱动网络编程
// ============================================================================
//
// 设计意图：这不仅仅是"一个 echo server"，而是要展示 Rust 在网络编程中
// 能够达到和 C 一样的底层控制力，同时提供编译期安全保证。
//
// 关键设计对比：
// - C: 你手写 epoll_ctl/epoll_wait → 完全控制 → 但没有安全网
// - Java: Selector 封装了 epoll → 有安全网 → 但无法绕过抽象层
// - Go: netpoller 完全隐藏 epoll → 最省心 → 但失去控制力
// - Rust: mio 封装了 epoll/IOCP/kqueue → 有安全网 → 但你可以直接调 libc 绕过
//
// mio 在这里的角色：
// - 不是"隐藏 epoll"，而是"统一 epoll/IOCP/kqueue 的接口"
// - 在 Linux 上，mio::Poll 内部就是 epoll_create1 + epoll_ctl + epoll_wait
// - 在 Windows 上，mio::Poll 内部是 CreateIoCompletionPort + GetQueuedCompletionStatus
// - 你仍然知道底层在用什么，你仍然控制 register/deregister 的时机
//
// 本代码在 Windows 和 Linux 上都能编译运行，因为 mio 做了平台适配。

use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::io::{Read, Write};

// ============================================================================
// Token 设计：事件标识符
// ============================================================================
//
// 设计意图：mio 用 Token (实际上是 u64) 来标识哪个 fd 产生了事件。
// 这和 C 的 epoll_event.data.fd 是同一个概念。
//
// C 写法：
//   struct epoll_event ev;
//   ev.data.fd = listen_fd;  // 把 fd 塞进 epoll_event
//   epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &ev);
//
// Rust 写法：
//   poll.registry().register(&mut server, Token(0), Interest::READABLE);
//
// 区别：C 用原始 fd 作为标识，Rust 用 Token 抽象。Token 可以是任意 u64，
// 你可以把 fd 编码进去（和 C 一样），也可以用自定义索引（更灵活）。

// Token(0) 留给监听 socket
const SERVER_TOKEN: Token = Token(0);
// 连接 token 从 1 开始递增
const FIRST_CLIENT_TOKEN: usize = 1;

// ============================================================================
// 连接状态管理
// ============================================================================
//
// 设计意图：在 C 中，连接状态通常是一个 struct，包含 fd 和 buffer。
// 问题在于 C 不会帮你管理这些状态的生命周期——如果你 close(fd) 了
// 但忘了从状态表里删除，就会出现 use-after-free。
//
// Rust 的 HashMap<Token, Connection> 在这里提供了安全保障：
// - 当 Connection 被 drop 时，其内部的 TcpStream 自动关闭
// - 你从 HashMap 中 remove 一个 Connection，它就被 drop 了
// - 不可能出现"fd 已关但状态还在"的情况
//
// 对比 Go：
//   Go 的 net.Conn 也有类似保障（GC 自动关闭），但时机不确定（GC 可能延迟）
//   Rust 的 drop 是确定性的，离开作用域立即关闭

struct Connection {
    // TcpStream 拥有底层 fd，drop 时自动 close
    stream: TcpStream,
    // 读缓冲区
    read_buf: [u8; 1024],
    // 写缓冲区（echo 模式下，读到的数据原样写回）
    write_buf: Vec<u8>,
}

impl Connection {
    fn new(stream: TcpStream) -> Self {
        Connection {
            stream,
            read_buf: [0u8; 1024],
            write_buf: Vec::new(),
        }
    }

    // ========================================================================
    // 非阻塞读：和 C 的 read() + EAGAIN 处理等价
    // ========================================================================
    //
    // C 写法：
    //   int n = read(fd, buf, sizeof(buf));
    //   if (n < 0) {
    //       if (errno == EAGAIN || errno == EWOULDBLOCK) {
    //           // 没数据可读，正常返回
    //       } else {
    //           perror("read");
    //       }
    //   } else if (n == 0) {
    //       // 对端关闭连接
    //   }
    //
    // Rust 的mio::net::TcpStream::read 在非阻塞模式下：
    //   - 返回 Ok(n) 表示读了 n 字节
    //   - 返回 Err(kind = WouldBlock) 等价于 EAGAIN
    //   - 返回 Ok(0) 表示对端关闭
    //
    // 关键区别：C 用 errno（全局变量，线程不安全），Rust 用 Result（值传递，线程安全）

    fn handle_read(&mut self) -> bool {
        match self.stream.read(&mut self.read_buf) {
            Ok(0) => {
                // 对端关闭连接 → 返回 false 表示需要清理
                // C 中你经常忘记处理这个 → fd 泄漏
                // Rust 的返回值设计让你必须处理
                println!("  → Connection closed by peer");
                false
            }
            Ok(n) => {
                println!("  → Read {} bytes", n);
                // Echo：将读到的数据加入写缓冲
                self.write_buf.extend_from_slice(&self.read_buf[..n]);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 非阻塞模式下没有数据可读 → 正常情况，不做处理
                // 这和 C 的 EAGAIN 完全等价
                true
            }
            Err(e) => {
                println!("  → Read error: {}", e);
                false
            }
        }
    }

    // ========================================================================
    // 非阻塞写：和 C 的 write() + 部分写入处理等价
    // ========================================================================
    //
    // C 写法：
    //   int n = write(fd, buf + offset, remaining);
    //   if (n < 0) {
    //       if (errno == EAGAIN) { /* 缓冲区满，下次再写 */ }
    //       else { perror("write"); }
    //   } else {
    //       offset += n;
    //       if (offset == total) { /* 写完了 */ }
    //   }
    //
    // Rust 的优势：write_buf 是 Vec<u8>，自动管理写偏移，
    // 不需要手动追踪 offset（C 中常见 bug 来源）

    fn handle_write(&mut self) -> bool {
        if self.write_buf.is_empty() {
            return true;
        }

        match self.stream.write(&self.write_buf) {
            Ok(n) => {
                println!("  → Written {} bytes", n);
                // 移除已写入的数据（drain 比 C 的 memmove 更安全）
                self.write_buf.drain(..n);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 内核发送缓冲区满了 → 下次 EPOLLOUT 时再写
                true
            }
            Err(e) => {
                println!("  → Write error: {}", e);
                false
            }
        }
    }
}

// ============================================================================
// 主事件循环
// ============================================================================
//
// 这就是 epoll 事件循环的 Rust 版本。
//
// C 的事件循环结构：
//   while (1) {
//       int n = epoll_wait(epfd, events, MAX_EVENTS, -1);
//       for (int i = 0; i < n; i++) {
//           if (events[i].data.fd == listen_fd) { /* accept */ }
//           else if (events[i].events & EPOLLIN) { /* read */ }
//           else if (events[i].events & EPOLLOUT) { /* write */ }
//       }
//   }
//
// Rust 版本的差异：
// 1. poll.poll() = epoll_wait（Linux）/ GetQueuedCompletionStatus（Windows）
// 2. Token 替代 data.fd 作为标识
// 3. Interest::READABLE = EPOLLIN
// 4. Interest::WRITABLE = EPOLLOUT
// 5. HashMap<Token, Connection> 替代你自己管理的 fd → state 映射

fn main() -> std::io::Result<()> {
    println!("=== Rust Raw Socket I/O: Event-Driven Echo Server ===");
    println!("Platform: {}", if cfg!(target_os = "linux") { "Linux (mio → epoll)" }
                            else if cfg!(target_os = "windows") { "Windows (mio → IOCP)" }
                            else if cfg!(target_os = "macos") { "macOS (mio → kqueue)" }
                            else { "Unknown" });
    println!("Listening on 127.0.0.1:8080");
    println!("Connect with: nc 127.0.0.1 8080 (or telnet)");
    println!();

    // 1. 创建 Poll 实例
    //    Linux: 内部调用 epoll_create1(0) → 返回 epfd
    //    Windows: 内部调用 CreateIoCompletionPort → 返回完成端口句柄
    let mut poll = Poll::new()?;
    println!("[init] Poll created (epfd/iocp handle allocated)");

    // 2. 创建 TCP 监听 socket
    //    等价于 C:
    //      int fd = socket(AF_INET, SOCK_STREAM, 0);
    //      setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, ...);
    //      bind(fd, (struct sockaddr*)&addr, sizeof(addr));
    //      listen(fd, 128);
    //
    //    mio::net::TcpListener::bind 一步完成了 socket + bind + listen
    //    如果需要更细粒度控制，用 socket2 crate：
    //      let socket = socket2::Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP));
    //      socket.set_reuse_address(true)?;
    //      socket.bind(&addr.into())?;
    //      socket.listen(128)?;
    //      let listener = TcpListener::from_std(socket.into());
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;
    println!("[init] TCP listener bound to {}", addr);

    // 3. 注册监听 socket 到 poll
    //    Linux: epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &{EPOLLIN, data={listen_fd}})
    //    Windows: CreateIoCompletionPort(listen_handle, iocp_port, ...)
    poll.registry().register(
        &mut server,
        SERVER_TOKEN,
        Interest::READABLE,
    )?;
    println!("[init] Server registered with READABLE interest");
    println!();

    // 4. 事件缓冲区（和 C 的 struct epoll_event events[MAX_EVENTS] 等价）
    let mut events = Events::with_capacity(1024);

    // 5. 连接状态表
    //    C 中你通常用一个数组或哈希表：struct Connection conns[MAX_FDS];
    //    Rust 用 HashMap<Token, Connection>：自动管理生命周期，不会泄漏
    let mut connections: HashMap<Token, Connection> = HashMap::new();
    let mut next_token = FIRST_CLIENT_TOKEN;

    // ========================================================================
    // 主事件循环：等价于 C 的 while(1) { epoll_wait(...); ... }
    // ========================================================================
    println!("[loop] Entering event loop...");
    loop {
        // 等价于 C: int n = epoll_wait(epfd, events, 1024, -1);
        // poll() 会阻塞直到至少一个事件发生
        poll.poll(&mut events, None)?;

        for event in &events {
            // ==================================================================
            // Token 匹配：等价于 C 的 if (events[i].data.fd == listen_fd)
            // ==================================================================
            match event.token() {
                SERVER_TOKEN => {
                    // 监听 socket 可读 → 有新连接到达
                    // 等价于 C:
                    //   int client_fd = accept4(listen_fd, NULL, NULL, SOCK_NONBLOCK);
                    handle_new_connection(
                        &mut server, &mut poll, &mut connections, &mut next_token
                    )?;
                }
                token => {
                    // 已有连接的事件
                    handle_connection_event(
                        &mut poll, &mut connections, token, event
                    )?;
                }
            }
        }
    }
}

// ============================================================================
// 处理新连接
// ============================================================================
//
// 等价于 C:
//   int client_fd = accept4(listen_fd, (struct sockaddr*)&client_addr, &len, SOCK_NONBLOCK);
//   setnonblocking(client_fd);  // 如果不是 accept4
//   ev.events = EPOLLIN | EPOLLOUT;
//   ev.data.fd = client_fd;
//   epoll_ctl(epfd, EPOLL_CTL_ADD, client_fd, &ev);
//
// 关键区别：
// C: client_fd 是裸 int，你可以传给任何函数，随时可能被 close 而没有通知
// Rust: TcpStream 拥有 fd，所有权转移意味着只有一个持有者

fn handle_new_connection(
    server: &mut TcpListener,
    poll: &mut Poll,
    connections: &mut HashMap<Token, Connection>,
    next_token: &mut usize,
) -> std::io::Result<()> {
    // accept 可能返回多个连接（因为 epoll 是边缘触发时可能积累多个连接）
    // 不过 mio 默认是水平触发，每次 accept 一个即可
    loop {
        match server.accept() {
            Ok((stream, addr)) => {
                println!("[accept] New connection from {}", addr);

                let token = Token(*next_token);
                *next_token += 1;

                // 创建连接状态
                let mut conn = Connection::new(stream);

                // 注册到 poll —— 等价于 epoll_ctl(EPOLL_CTL_ADD)
                // 同时关注 READABLE（有数据到达）和 WRITABLE（可以发送数据）
                poll.registry().register(
                    &mut conn.stream,
                    token,
                    Interest::READABLE | Interest::WRITABLE,
                )?;

                connections.insert(token, conn);
                println!("[accept] Registered with token {:?}", token);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 没有更多待接受的连接了
                break;
            }
            Err(e) => {
                println!("[accept] Error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

// ============================================================================
// 处理连接事件
// ============================================================================
//
// 等价于 C:
//   if (events[i].events & EPOLLIN) { handle_read(fd); }
//   if (events[i].events & EPOLLOUT) { handle_write(fd); }
//
// Rust 用 event.is_readable() / event.is_writable() 替代位运算
// 但本质完全一样——检查事件类型，执行对应操作

fn handle_connection_event(
    poll: &mut Poll,
    connections: &mut HashMap<Token, Connection>,
    token: Token,
    event: &mio::event::Event,
) -> std::io::Result<()> {
    let mut should_remove = false;

    if let Some(conn) = connections.get_mut(&token) {
        // 可读事件 → 有数据到达
        if event.is_readable() {
            println!("[event] Readable on {:?}", token);
            if !conn.handle_read() {
                should_remove = true;
            }
        }

        // 可写事件 → 可以发送数据
        // 注意：只有当 write_buf 非空时才需要处理写事件
        if event.is_writable() && !should_remove {
            println!("[event] Writable on {:?}", token);
            if !conn.handle_write() {
                should_remove = true;
            }
        }
    }

    if should_remove {
        // ====================================================================
        // 清理连接：等价于 C 的 close(fd) + epoll_ctl(EPOLL_CTL_DEL)
        // ====================================================================
        //
        // C 写法：
        //   epoll_ctl(epfd, EPOLL_CTL_DEL, fd, NULL);
        //   close(fd);
        //   // 如果你忘了其中任何一步 → bug
        //
        // Rust 写法：
        //   deregister 告诉 poll 不再监听这个 fd
        //   HashMap::remove 返回 Connection，它离开作用域后被 drop
        //   Drop 自动 close fd
        //
        // 三步合一：deregister + remove + drop → 不可能遗漏

        if let Some(mut conn) = connections.remove(&token) {
            // deregister 等价于 epoll_ctl(EPOLL_CTL_DEL)
            let _ = poll.registry().deregister(&mut conn.stream);
            println!("[cleanup] Connection {:?} removed and fd closed", token);
        }
    }

    Ok(())
}

// ============================================================================
// 延伸阅读：如果要在 Linux 上直接调用 epoll（不用 mio）
// ============================================================================
//
// Cargo.toml:
//   [target.'cfg(target_os = "linux")'.dependencies]
//   libc = "0.2"
//
// 代码：
//   use libc::*;
//
//   fn raw_epoll_server() {
//       unsafe {
//           // 创建 epoll 实例
//           let epfd = epoll_create1(0);
//
//           // 创建监听 socket
//           let listen_fd = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0);
//
//           // bind + listen
//           let addr = sockaddr_in {
//               sin_family: AF_INET as u16,
//               sin_port: 8080u16.to_be(),
//               sin_addr: in_addr { s_addr: INADDR_LOOPBACK.to_be() },
//               sin_zero: [0; 8],
//           };
//           bind(listen_fd, &addr as *const _ as *const sockaddr, sizeof::<sockaddr_in>() as u32);
//           listen(listen_fd, 128);
//
//           // 注册到 epoll
//           let mut ev = epoll_event {
//               events: EPOLLIN as u32,
//               u64: listen_fd as u64,
//           };
//           epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &mut ev);
//
//           // 事件循环
//           let mut events = vec![epoll_event { events: 0, u64: 0 }; 1024];
//           loop {
//               let n = epoll_wait(epfd, events.as_mut_ptr(), 1024, -1);
//               for i in 0..n as usize {
//                   // 处理事件...
//               }
//           }
//       }
//   }
//
// 这段代码和 C 的 epoll 服务器几乎一模一样。
// 唯一的区别是 unsafe 块——Rust 明确标记了"这里编译器无法保证安全"。
//
// 而 mio 版本做的事完全相同，只是用安全的 API 包装了这些 unsafe 调用。
// 编译后的机器码和手写的 C 代码性能等价——这就是"零成本抽象"。
