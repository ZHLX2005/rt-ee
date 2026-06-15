// =============================================================================
// NIO State Machine: 手撕非阻塞 Socket 编程的核心
// =============================================================================
//
// 本程序演示 NIO 编程中最核心的设计模式——连接状态机。
//
// 设计对比：
//
// C 的 NIO：
//   struct conn_state { int fd; char buf[4096]; int offset; int total; };
//   → 手写 struct，手动追踪偏移量，double-close / use-after-free 随时可能发生
//
// Java NIO：
//   SelectionKey key = ...;
//   State state = (State) key.attachment();  // 强制转换，运行时才能发现错误
//   ByteBuffer buf = ...; buf.flip();        // flip/clear 易错
//
// Rust NIO：
//   enum ConnState { Reading { buf: Vec<u8> }, Writing { buf: Vec<u8>, written: usize } }
//   → 编译期穷尽检查，不可能遗漏状态
//   → buf 和 written 绑定在 enum variant 中，不可能不一致
//   → TcpStream 拥有 fd，Drop 自动 close

use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::io::{Read, Write};

// ============================================================================
// Token 设计
// ============================================================================
// Token(0) 留给 server
const SERVER: Token = Token(0);
const START_TOKEN: usize = 1;

// ============================================================================
// 连接状态机（核心）
// ============================================================================
//
// 这是 NIO 编程的灵魂。每个非阻塞连接都是一个状态机。
//
// C 的做法：
//   int state;  // 0=idle, 1=reading, 2=writing, 3=closed
//   // state 是 int → 你可以设成任意值，编译器不管
//   // 忘了处理某个状态 → 运行时才挂
//
// Java NIO 的做法：
//   Object attachment = key.attachment();
//   if (attachment instanceof ReadingState) { ... }
//   else if (attachment instanceof WritingState) { ... }
//   // 忘了 instanceof 检查 → ClassCastException
//
// Rust 的做法：
//   match state {
//       ConnState::Reading { .. } => { ... }
//       ConnState::Writing { .. } => { ... }
//   }
//   // 忘了处理某个 variant → 编译错误！
//
// 这就是类型状态模式（Typestate Pattern）在网络编程中的威力。

#[derive(Debug)]
enum ConnState {
    /// 正在从 socket 读取数据
    Reading {
        buf: Vec<u8>,
    },
    /// 有待发送的数据（partial write 状态）
    Writing {
        buf: Vec<u8>,
        /// 已经写了多少字节——这在 C 中是一个独立的 int，很容易和 buf 脱节
        /// 在 Rust 中它绑定在 enum variant 里，不可能不一致
        written: usize,
    },
    /// 对端关闭了连接（收到 FIN，read 返回 0）
    HalfClosed,
}

// 事件处理后的动作
enum Action {
    /// 不需要做什么
    None,
    /// 需要重新注册 Interest（动态调整关注的事件类型）
    Reregister(Interest),
    /// 关闭连接
    Close,
}

// ============================================================================
// Connection：封装 TcpStream + 状态机
// ============================================================================

struct Connection {
    stream: TcpStream,
    state: ConnState,
    token: Token,
    /// 统计：总共读取的字节数
    total_read: usize,
    /// 统计：总共写入的字节数
    total_written: usize,
}

impl Connection {
    fn new(stream: TcpStream, token: Token) -> Self {
        println!("  [{}] New connection → Reading state", token.0);
        Connection {
            stream,
            state: ConnState::Reading {
                buf: vec![0u8; 4096],
            },
            token,
            total_read: 0,
            total_written: 0,
        }
    }

    // ========================================================================
    // 事件分发：根据当前状态 + 事件类型决定做什么
    // ========================================================================
    //
    // 这是 NIO 状态机的核心逻辑。
    // C 的做法：if (state == READING && (events & EPOLLIN)) { ... }
    // Java：    if (key.isReadable()) { ... }
    // Rust：    match (state, event_type) { ... }
    //
    // Rust 的 match 保证了穷尽性——你不可能忘记处理某个状态。

    fn handle_event(&mut self, event: &mio::event::Event) -> Action {
        // 先处理可写（如果状态是 Writing，可写优先级高于可读）
        if event.is_writable() {
            if let Action::Close = self.do_write() {
                return Action::Close;
            }
        }

        // 再处理可读
        if event.is_readable() {
            match self.do_read() {
                Action::Close => return Action::Close,
                Action::Reregister(interest) => return Action::Reregister(interest),
                Action::None => {}
            }
        }

        // 检查半关闭状态
        if matches!(self.state, ConnState::HalfClosed) {
            return Action::Close;
        }

        // 根据当前状态决定 Interest
        // 这是避免事件空转的关键：只在需要时关注 WRITABLE
        match &self.state {
            ConnState::Reading { .. } => Action::Reregister(Interest::READABLE),
            ConnState::Writing { .. } => {
                Action::Reregister(Interest::READABLE | Interest::WRITABLE)
            }
            ConnState::HalfClosed => Action::Close,
        }
    }

    // ========================================================================
    // 非阻塞读
    // ========================================================================
    //
    // 与 C 的对比：
    //   C:     n = read(fd, buf, sizeof(buf));
    //          if (n < 0 && errno == EAGAIN) → 正常
    //          if (n == 0) → 对端关闭
    //
    //   Rust:  n = stream.read(&mut buf);
    //          Err(WouldBlock) → 正常（等价于 EAGAIN）
    //          Ok(0)          → 对端关闭
    //
    //   Java:  n = channel.read(byteBuffer);
    //          byteBuffer.flip();  ← 容易忘
    //
    // 关键区别：
    //   C 用全局 errno → 线程不安全
    //   Rust 用 Result<Ok, Err> → 值传递，线程安全
    //   Java 用 ByteBuffer.flip() → 忘了就白读

    fn do_read(&mut self) -> Action {
        let token = self.token;

        match &mut self.state {
            ConnState::Reading { buf } => {
                // 清空缓冲区准备读取
                buf.fill(0);

                match self.stream.read(buf) {
                    Ok(0) => {
                        // 对端关闭连接（收到 FIN）
                        // C 经常忘记处理这个 → fd 泄漏
                        // Rust 的返回值设计让你必须处理
                        println!("  [{}] Read 0 bytes → HalfClosed (peer closed)", token.0);
                        self.state = ConnState::HalfClosed;
                        Action::Close
                    }
                    Ok(n) => {
                        self.total_read += n;
                        let data = &buf[..n];

                        // 尝试转为字符串显示
                        let text = std::str::from_utf8(data)
                            .map(|s| s.trim_end())
                            .unwrap_or("<binary>");

                        if text.is_empty() || text.len() < 50 {
                            println!("  [{}] Read {} bytes: {:?}", token.0, n, text);
                        } else {
                            println!("  [{}] Read {} bytes: {:?}...", token.0, n, &text[..50]);
                        }

                        // 状态转换：Reading → Writing
                        // 把读到的数据原封不动写回（echo）
                        //
                        // Rust enum 的优势：
                        // - 旧状态（Reading）的数据被消费
                        // - 新状态（Writing）的数据被创建
                        // - 不可能出现"旧 buf 和新 buf 混用"的 bug
                        let write_data = data.to_vec();
                        self.state = ConnState::Writing {
                            buf: write_data,
                            written: 0,
                        };

                        // 有数据要写 → 同时关注 READABLE 和 WRITABLE
                        // 这是防止事件空转的关键
                        Action::Reregister(Interest::READABLE | Interest::WRITABLE)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // 非阻塞模式下没有数据可读 → 正常，等下次通知
                        // 等价于 C 的 errno == EAGAIN
                        Action::None
                    }
                    Err(e) => {
                        println!("  [{}] Read error: {} → Close", token.0, e);
                        Action::Close
                    }
                }
            }
            ConnState::Writing { .. } => {
                // 正在写状态，读事件暂时忽略（下次再处理）
                // 这在 NIO 中很常见：读写事件可能同时到达，需要优先级排序
                Action::None
            }
            ConnState::HalfClosed => {
                Action::Close
            }
        }
    }

    // ========================================================================
    // 非阻塞写（处理 Partial Write）
    // ========================================================================
    //
    // Partial Write 是 NIO 编程中最容易出 bug 的地方：
    //
    // 你想写 100 字节：
    //   write(buf[0..100]) → 返回 30     // 只写了 30 字节
    //   write(buf[30..100]) → 返回 EAGAIN // 内核缓冲区满了
    //   等待 EPOLLOUT...
    //   write(buf[30..100]) → 返回 50     // 又写了 50 字节
    //   write(buf[80..100]) → 返回 20     // 全部写完
    //
    // C 的做法：
    //   int written = 0;
    //   while (written < total) {
    //       int n = write(fd, buf + written, total - written);
    //       if (n < 0 && errno == EAGAIN) break; // 记住 written，下次继续
    //       written += n;
    //   }
    //   → written 和 buf 的关系全靠程序员纪律
    //
    // Java 的做法：
    //   channel.write(byteBuffer);
    //   if (byteBuffer.hasRemaining()) { /* 没写完，等下次 */ }
    //   → ByteBuffer 的 position/limit/compact 容易搞混
    //
    // Rust 的做法：
    //   ConnState::Writing { buf, written } → written 绑定在 enum 中
    //   → 不可能和 buf 脱节

    fn do_write(&mut self) -> Action {
        let token = self.token;

        match &mut self.state {
            ConnState::Writing { buf, written } => {
                let remaining = &buf[*written..];

                if remaining.is_empty() {
                    // 已经写完了，不应该进入这个分支
                    // 但防御性编程还是需要的
                    return Action::None;
                }

                match self.stream.write(remaining) {
                    Ok(n) => {
                        *written += n;
                        self.total_written += n;
                        println!(
                            "  [{}] Written {}/{} bytes",
                            token.0, *written, buf.len()
                        );

                        if *written >= buf.len() {
                            // 全部写完！
                            // 状态转换：Writing → Reading
                            //
                            // 重要：写完后只关注 READABLE，取消 WRITABLE
                            // 否则 epoll 会不断通知"可写"→ 事件循环空转 → CPU 100%
                            //
                            // 这是 NIO 最常见的性能陷阱：
                            //   C: 忘了 epoll_ctl(MOD) 取消 EPOLLOUT → CPU 100%
                            //   Java: 忘了 key.interestOps(& ~OP_WRITE) → CPU 100%
                            //   Rust: Action::Reregister(READABLE) → 只关注可读
                            println!(
                                "  [{}] Write complete → Reading state (total: read={}, written={})",
                                token.0, self.total_read, self.total_written
                            );
                            self.state = ConnState::Reading {
                                buf: vec![0u8; 4096],
                            };
                        }
                        Action::None // Interest 会在 handle_event 中统一设置
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // 内核发送缓冲区满了 → 等下次 EPOLLOUT 事件
                        // 这就是 partial write：written < buf.len()
                        println!(
                            "  [{}] Write WouldBlock ({}/{} written, waiting for WRITABLE)",
                            token.0, *written, buf.len()
                        );
                        Action::None
                    }
                    Err(e) => {
                        println!("  [{}] Write error: {} → Close", token.0, e);
                        Action::Close
                    }
                }
            }
            _ => Action::None,
        }
    }
}

// ============================================================================
// 主事件循环
// ============================================================================

fn main() -> std::io::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  NIO State Machine: Rust 手撕非阻塞 Socket 编程              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  演示：                                                       ║");
    println!("║  1. 连接状态机 (enum ConnState)                               ║");
    println!("║  2. Partial Write 处理                                        ║");
    println!("║  3. 动态 Interest 调整（防止事件空转）                          ║");
    println!("║  4. Reading ↔ Writing 状态转换                                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    let platform = if cfg!(target_os = "linux") { "Linux (mio → epoll)" }
                   else if cfg!(target_os = "windows") { "Windows (mio → IOCP)" }
                   else if cfg!(target_os = "macos") { "macOS (mio → kqueue)" }
                   else { "Unknown" };
    println!("║  Platform: {:<49}║", platform);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // 1. 创建 Poll（等价于 epoll_create1 / CreateIoCompletionPort）
    let mut poll = Poll::new()?;

    // 2. 创建 TCP listener（等价于 socket + bind + listen）
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;

    // 3. 注册 server 到 poll（等价于 epoll_ctl ADD, EPOLLIN）
    //    只关注 READABLE（有新连接到达时 listen fd 变为可读）
    poll.registry().register(
        &mut server,
        SERVER,
        Interest::READABLE,
    )?;

    println!("[server] Listening on {}", addr);
    println!("[server] Connect with: nc 127.0.0.1 8080");
    println!();

    // 4. 事件缓冲区和连接表
    let mut events = Events::with_capacity(1024);
    let mut connections: HashMap<Token, Connection> = HashMap::new();
    let mut next_token = START_TOKEN;

    // ====================================================================
    // 主事件循环（等价于 C 的 while(1) { epoll_wait(...) }）
    // ====================================================================
    println!("[loop] Entering event loop...");
    println!("───────────────────────────────────────────────────");

    loop {
        // 等价于 C: epoll_wait(epfd, events, max_events, -1);
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                SERVER => {
                    // ==================================================
                    // 新连接到达
                    // ==================================================
                    // 循环 accept 直到 WouldBlock（ET 模式的标准做法）
                    // mio 默认 LT，但循环 accept 仍然好习惯
                    loop {
                        match server.accept() {
                            Ok((stream, addr)) => {
                                let token = Token(next_token);
                                next_token += 1;

                                println!("[accept] Connection from {} → token={}", addr, token.0);

                                // 创建连接状态机
                                let mut conn = Connection::new(stream, token);

                                // 初始 Interest：READABLE
                                // 注意：不关注 WRITABLE（避免事件空转）
                                // 这和 C 的 EPOLLIN 但不加 EPOLLOUT 一样
                                poll.registry().register(
                                    &mut conn.stream,
                                    token,
                                    Interest::READABLE,
                                )?;

                                connections.insert(token, conn);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                break; // 没有更多待接受的连接
                            }
                            Err(e) => {
                                eprintln!("[accept] Error: {}", e);
                                break;
                            }
                        }
                    }
                }
                token => {
                    // ==================================================
                    // 已有连接的事件
                    // ==================================================
                    // 这是状态机运转的地方

                    let action = if let Some(conn) = connections.get_mut(&token) {
                        conn.handle_event(event)
                    } else {
                        continue;
                    };

                    match action {
                        Action::Reregister(interest) => {
                            // 动态调整 Interest
                            // 等价于 C: epoll_ctl(epfd, EPOLL_CTL_MOD, fd, &new_ev);
                            // 等价于 Java: key.interestOps(newOps);
                            if let Some(conn) = connections.get_mut(&token) {
                                let _ = poll.registry().reregister(
                                    &mut conn.stream,
                                    token,
                                    interest,
                                );
                            }
                        }
                        Action::Close => {
                            // 清理连接
                            // 等价于 C: epoll_ctl(DEL) + close(fd)
                            // Rust 的优势：三步合一
                            //   1. deregister → 告诉 poll 不再监听
                            //   2. remove from HashMap → Connection 被取出
                            //   3. Drop → fd 自动 close
                            if let Some(mut conn) = connections.remove(&token) {
                                let _ = poll.registry().deregister(&mut conn.stream);
                                println!(
                                    "  [{}] Closed (total: read={}, written={})",
                                    token.0, conn.total_read, conn.total_written
                                );
                            }
                        }
                        Action::None => {}
                    }
                }
            }
        }
    }
}
