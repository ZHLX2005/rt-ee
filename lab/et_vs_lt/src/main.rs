// =============================================================================
// ET vs LT: 边缘触发 vs 水平触发 的代码差异
// =============================================================================
//
// 核心区别只有一句话：
//   LT: 可以只读一次，没读完下次还会通知
//   ET: 必须循环读到 EAGAIN，否则永远不再通知
//
// 代码差异只有两处：
//   1. 注册时多一个 flag：EPOLLET
//   2. 处理时多一个 loop：while (read() != EAGAIN)
//
// 本程序用 mio (LT) 实现一个 echo server，
// 在每个关键位置用注释展示 ET 模式的写法。

use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::io::{Read, Write};

const SERVER: Token = Token(0);
const START: usize = 1;

// ============================================================================
// 连接状态
// ============================================================================

struct Connection {
    stream: TcpStream,
    read_buf: [u8; 4096],
    write_buf: Vec<u8>,
    token: Token,
}

// ============================================================================
// LT vs ET 的核心区别体现在这两个函数中
// ============================================================================

impl Connection {
    fn new(stream: TcpStream, token: Token) -> Self {
        Connection {
            stream,
            read_buf: [0u8; 4096],
            write_buf: Vec::new(),
            token,
        }
    }

    // ========================================================================
    // LT 模式：accept 一次就够
    // ========================================================================
    // LT 的语义：如果还有未 accept 的连接，下次 epoll_wait 还会通知
    // 所以你可以只 accept 一次就返回
    //
    // ET 模式的写法（对比）：
    //   loop {
    //       match server.accept() {
    //           Ok((stream, addr)) => { ... }
    //           Err(e) if e.kind() == WouldBlock => break,  // EAGAIN → 取完了
    //           Err(e) => break,
    //       }
    //   }
    // 差异：加一个 loop，处理 EAGAIN

    // ========================================================================
    // LT 模式的 read：读一次就够
    // ========================================================================
    //
    // 这是 LT 模式的写法。mio 默认就是 LT。
    //
    // LT 的语义：如果 socket 上还有未读数据，下次 epoll_wait 还会通知
    // 所以你可以只 read 一次就返回

    fn handle_readable_lt(&mut self) -> bool {
        // ★ LT：只读一次 ★
        match self.stream.read(&mut self.read_buf) {
            Ok(0) => {
                println!("  [LT][{}] read 0 → peer closed", self.token.0);
                return false;
            }
            Ok(n) => {
                let text = std::str::from_utf8(&self.read_buf[..n])
                    .map(|s| s.trim_end())
                    .unwrap_or("<binary>");
                println!("  [LT][{}] read {} bytes: {:?}", self.token.0, n, text);
                self.write_buf.extend_from_slice(&self.read_buf[..n]);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // LT 模式下这通常不会发生（因为 LT 只在有数据时通知）
                // 但防御性编程总是好的
                true
            }
            Err(e) => {
                println!("  [LT][{}] read error: {}", self.token.0, e);
                false
            }
        }
        // 没读完的数据？不用担心，下次 poll.poll() 还会通知
    }

    // ========================================================================
    // ET 模式的 read：必须循环到 EAGAIN（用注释展示，实际在 Linux 上运行）
    // ========================================================================
    //
    // ET 的语义：只在数据从无到有时通知一次，之后不管了
    // 所以你必须读完所有数据，否则剩余数据永远不会被通知
    //
    // ★★★ 这就是 ET 和 LT 在代码层面的唯一本质区别 ★★★

    fn handle_readable_et_pattern(&mut self) -> bool {
        // ★ ET：必须循环读到 EAGAIN ★
        let mut total_read = 0;
        let max_reads = 64; // 防饥饿：限制单次事件处理的最大读次数

        for round in 0..max_reads {
            match self.stream.read(&mut self.read_buf) {
                Ok(0) => {
                    println!("  [ET][{}] read 0 (round {}) → peer closed",
                             self.token.0, round);
                    return false;
                }
                Ok(n) => {
                    total_read += n;
                    let text = std::str::from_utf8(&self.read_buf[..n])
                        .map(|s| s.trim_end())
                        .unwrap_or("<binary>");
                    if round == 0 {
                        println!("  [ET][{}] read {} bytes (round {}): {:?}",
                                 self.token.0, n, round, text);
                    }
                    self.write_buf.extend_from_slice(&self.read_buf[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // ★ EAGAIN → 所有数据读完了，这就是 ET 循环的退出条件 ★
                    // LT 不需要这个循环，因为没读完下次还会通知
                    // ET 必须在这里读完，否则永远不再通知
                    println!("  [ET][{}] EAGAIN after {} rounds, {} bytes total",
                             self.token.0, round, total_read);
                    break;
                }
                Err(e) => {
                    println!("  [ET][{}] read error: {}", self.token.0, e);
                    return false;
                }
            }
        }

        if total_read > 0 {
            println!("  [ET][{}] total read: {} bytes", self.token.0, total_read);
        }
        true
    }

    // ========================================================================
    // write（LT 和 ET 的写法基本相同，因为都需要处理 partial write）
    // ========================================================================

    fn handle_writable(&mut self) -> bool {
        if self.write_buf.is_empty() {
            return true;
        }

        match self.stream.write(&self.write_buf) {
            Ok(n) => {
                println!("  [{}][{}] written {}/{} bytes",
                    if cfg!(target_os = "linux") { "ET" } else { "LT" },
                    self.token.0, n, self.write_buf.len());
                self.write_buf.drain(..n);
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                true // 内核缓冲区满，等下次 WRITABLE
            }
            Err(e) => {
                println!("  [{}][{}] write error: {}",
                    if cfg!(target_os = "linux") { "ET" } else { "LT" },
                    self.token.0, e);
                false
            }
        }
    }
}

// ============================================================================
// 主函数：启动 echo server
// ============================================================================

fn main() -> std::io::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  ET vs LT: 边缘触发 vs 水平触发 — 代码差异对照              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    // mio 始终使用 Level Triggered 模式
    // 这里展示的是 LT 模式的代码，ET 模式的区别在注释中标注
    println!("║  当前模式: LT (mio 默认)                                    ║");
    println!("║  ET 模式的差异用 [ET] 标注在注释中                           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  核心区别（就这两处）：                                       ║");
    println!("║                                                              ║");
    println!("║  注册: EPOLLIN        vs  EPOLLIN | EPOLLET                  ║");
    println!("║  读:   read一次       vs  loop {{ read到EAGAIN }}              ║");
    println!("║  accept: accept一次   vs  loop {{ accept到EAGAIN }}            ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut poll = Poll::new()?;
    let addr = "127.0.0.1:8081".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;

    // 注册 server
    // LT 写法（mio 默认）：
    poll.registry().register(&mut server, SERVER, Interest::READABLE)?;
    //
    // ET 写法（如果用 raw epoll）：
    //   let mut ev = libc::epoll_event {
    //       events: (libc::EPOLLIN | libc::EPOLLET) as u32,  // ← 多一个 EPOLLET
    //       u: libc::epoll_data { fd: listen_fd },
    //   };
    //   libc::epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &mut ev);

    println!("[server] Listening on {}", addr);
    println!("[server] Test: nc 127.0.0.1 8081");
    println!();

    let mut events = Events::with_capacity(1024);
    let mut connections: HashMap<Token, Connection> = HashMap::new();
    let mut next_token = START;

    println!("[loop] Event loop started");
    println!("─────────────────────────────────────");

    loop {
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                SERVER => {
                    // ==================================================
                    // accept：LT vs ET
                    // ==================================================
                    //
                    // LT：可以只 accept 一次
                    //     if let Ok((stream, addr)) = server.accept() { ... }
                    //
                    // ET：必须循环 accept 到 EAGAIN
                    //     loop {
                    //         match server.accept() {
                    //             Ok((stream, addr)) => { ... }
                    //             Err(WouldBlock) => break,  // ← 关键！
                    //             Err(e) => break,
                    //         }
                    //     }
                    //
                    // 实际上，即使是 LT 模式，循环 accept 也是好习惯
                    // 因为一次 epoll_wait 可能对应多个新连接
                    loop {
                        match server.accept() {
                            Ok((stream, addr)) => {
                                let token = Token(next_token);
                                next_token += 1;
                                println!("[accept] {} → token={}", addr, token.0);

                                let mut conn = Connection::new(stream, token);

                                // 注册 client：LT 写法
                                poll.registry().register(
                                    &mut conn.stream,
                                    token,
                                    Interest::READABLE,
                                )?;
                                // ET 写法：
                                //   events = EPOLLIN | EPOLLET
                                //   而且 fd 必须是 non-blocking

                                connections.insert(token, conn);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                break; // 没有更多待接受的连接
                            }
                            Err(e) => {
                                eprintln!("[accept] error: {}", e);
                                break;
                            }
                        }
                    }
                }
                token => {
                    let mut should_close = false;

                    if let Some(conn) = connections.get_mut(&token) {
                        if event.is_readable() {
                            // ==========================================
                            // read：这里展示 LT vs ET 的写法差异
                            // ==========================================

                            // 方式 1：LT 模式（只读一次）
                            // if !conn.handle_readable_lt() {
                            //     should_close = true;
                            // }

                            // 方式 2：ET 模式的写法（循环读到 EAGAIN）
                            // ★ 即使在 LT 模式下，用 ET 的写法也是安全的 ★
                            // ★ 区别只是：LT 不循环也没事，ET 不循环就丢数据 ★
                            if !conn.handle_readable_et_pattern() {
                                should_close = true;
                            }
                        }

                        if event.is_writable() && !should_close {
                            if !conn.handle_writable() {
                                should_close = true;
                            }
                        }
                    }

                    if should_close {
                        if let Some(mut conn) = connections.remove(&token) {
                            let _ = poll.registry().deregister(&mut conn.stream);
                            println!("  [{}] Connection closed", token.0);
                        }
                    } else if let Some(conn) = connections.get_mut(&token) {
                        // 动态调整 Interest
                        // 这是 LT 和 ET 都需要的——防止 WRITABLE 空转
                        let interest = if conn.write_buf.is_empty() {
                            Interest::READABLE
                        } else {
                            Interest::READABLE | Interest::WRITABLE
                        };
                        let _ = poll.registry().reregister(
                            &mut conn.stream, token, interest
                        );
                    }
                }
            }
        }
    }
}
