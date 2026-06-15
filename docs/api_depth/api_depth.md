# Rust API 深度：比 Go/Java 更底层体现在哪里？

## 设计背景

### 问题本质

不是"Rust 有多少 API"（Java JDK 有 4000+ 类，远超 Rust std），而是"**API 能控制到什么深度**"。

```
Java/Go 的 API 是一张宽而浅的桌子：覆盖面广，但探不到底层
Rust  的 API 是一根窄而深的钻头：覆盖面窄，但能直接钻到硬件
```

**核心结论：Rust 的 API 深度 = C 的深度 + 安全抽象。Go/Java 的 API 深度被 runtime/GC 截断了。**

---

## API 深度对比：7 个维度

### 维度 1：内存控制

#### Java — 完全不可控

```java
// Java：你无法控制内存
Object obj = new Object();  // 分配在堆上，你无法选择
// 没有栈分配
// 没有 placement new
// 没有手动释放
// 没有 uninitialized memory
// ByteBuffer.allocateDirect() 是唯一接近底层的 API，但受 GC 管理
```

#### Go — 有限的控制

```go
// Go：逃逸分析决定栈/堆，你无法强制
type Conn struct {
    fd int
    buf []byte
}
c := Conn{}  // 可能在栈上，也可能逃逸到堆上——你无法控制

// 没有 uninitialized memory
// 没有 custom allocator
// unsafe.Pointer 可以做但极少使用
```

#### Rust — 完全控制

```rust
use std::alloc::{alloc, dealloc, Layout, GlobalAlloc};

// 1. 栈分配（默认）
let conn = Connection { fd: 3, buf: [0u8; 4096] };  // 确定在栈上

// 2. 堆分配（显式选择）
let conn = Box::new(Connection { fd: 3, buf: vec![0; 4096] });

// 3. 未初始化内存（零开销初始化）
let mut buf: MaybeUninit<[u8; 4096]> = MaybeUninit::uninit();
// 不会初始化为 0——省了 memset 的开销
// Java 的 new byte[4096] 必须初始化为 0（JVM 安全保证）

// 4. 自定义分配器
struct MyAllocator;
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // 你可以：用 arena、用 slab、用 jemalloc、用 mimalloc
        // Java/Go 不可能做到这一点
        todo!()
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}

// 5. 手动内存布局控制
#[repr(C)]        // C ABI 兼容的内存布局
#[repr(packed)]   // 无对齐填充
#[repr(align(16))] // 16 字节对齐（SIMD 需要）
struct Packet {
    header: u16,
    payload: [u8; 0],  // 零大小类型（柔性数组）
}
```

**深度对比**：

| 能力 | Rust | Java | Go |
|------|------|------|-----|
| 栈/堆选择 | ✅ `Box` vs 栈 | ❌ JVM 决定 | ❌ 逃逸分析决定 |
| 未初始化内存 | ✅ `MaybeUninit` | ❌ 强制初始化 | ❌ 强制零初始化 |
| 自定义分配器 | ✅ `GlobalAlloc` | ❌ 不可能 | ❌ 不可能 |
| 内存布局控制 | ✅ `#[repr]` | ❌ JVM 控制 | ❌ 编译器控制 |
| 手动释放 | ✅ `drop()` / `ManuallyDrop` | ❌ GC | ❌ GC |
| 零拷贝 | ✅ `ptr::copy_nonoverlapping` | ❌ | ⚠️ 有限 |

---

### 维度 2：指针操作

#### Java — 没有指针

```java
// Java 没有 pointer 类型
// Unsafe 类曾经提供类似功能，但已被标记废弃
// sun.misc.Unsafe 不属于公共 API
```

#### Go — 受限的 unsafe.Pointer

```go
// Go：unsafe.Pointer 可以做但受限
var p unsafe.Pointer
// 不能做指针运算（直接）
// 不能 cast 到任意类型（有规则限制）
// 使用 unsafe 意味着放弃兼容性保证
```

#### Rust — 完整的指针体系

```rust
// Rust 有 4 种指针，精度从高到低

// 1. 引用 &T / &mut T —— 编译期安全，不可能悬空
let r: &i32 = &value;

// 2. Box<T> —— 堆上唯一所有权
let b: Box<i32> = Box::new(42);

// 3. *const T / *mut T —— 裸指针，可空，可悬空
let raw: *mut i32 = &mut value as *mut i32;
unsafe { *raw = 99; }

// 4. NonNull<T> —— 保证非空的裸指针
let nn: NonNull<i32> = unsafe { NonNull::new_unchecked(raw) };

// 指针运算
let ptr = buf.as_mut_ptr();
let offset_ptr = unsafe { ptr.add(4) };      // 前进 4 个元素
let val = unsafe { ptr.read_unaligned() };    // 非对齐读取
unsafe { ptr.write_volatile(42) };            // volatile 写（设备寄存器）

// 类型转换（transmute —— 最危险的零成本类型转换）
let bits: u32 = unsafe { std::mem::transmute(3.14f32) };
```

**为什么这很重要？**

操作系统内核、驱动、嵌入式、高性能网络栈——这些都**必须**用裸指针。Java/Go 在这些领域完全无法使用。

---

### 维度 3：并发原语

#### Java — 高层封装

```java
// Java 有丰富的并发 API，但都是高层封装
synchronized (lock) { ... }                    // 内置锁
ReentrantLock lock = new ReentrantLock();      // 可重入锁
AtomicInteger counter = new AtomicInteger(0);  // 原子操作
ExecutorService pool = Executors.newFixedThreadPool(8);  // 线程池

// 问题：
// - synchronized 无法控制内存屏障
// - AtomicInteger 无法选择 memory ordering
// - 你无法实现 Lock-Free 数据结构的高效版本（没有 compare_exchange）
```

#### Go — CSP 模型

```go
// Go 的并发以 channel 为核心
ch := make(chan int, 100)
go func() { ch <- 42 }()

// sync.Mutex, sync.WaitGroup, sync.Atomic
// 但没有 memory ordering 控制
// 没有 compare_exchange
// channel 内部是 runtime 管理的，你无法控制实现
```

#### Rust — 从硬件原语到高层抽象

```rust
use std::sync::atomic::{AtomicU32, Ordering};

// 1. 原子操作 + 精确的 memory ordering
static COUNTER: AtomicU32 = AtomicU32::new(0);

// Relaxed：无顺序保证（最快）
COUNTER.fetch_add(1, Ordering::Relaxed);

// Acquire/Release：生产者-消费者模式
COUNTER.store(42, Ordering::Release);  // 发布
let val = COUNTER.load(Ordering::Acquire);  // 获取

// SeqCst：顺序一致性（最安全，最慢）
COUNTER.compare_exchange(
    0,    // 期望值
    1,    // 新值
    Ordering::SeqCst,  // 成功时的 ordering
    Ordering::SeqCst,  // 失败时的 ordering
);

// 2. 自旋锁（用原子操作实现）
struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    fn lock(&self) {
        while self.locked.compare_exchange_weak(
            false, true,
            Ordering::Acquire,
            Ordering::Relaxed,
        ).is_err() {
            std::hint::spin_loop();  // CPU hint：告诉 CPU 这是在自旋
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

// Java 不可能实现这么精确的自旋锁——没有 compare_exchange + ordering

// 3. 条件变量（Condvar）—— 操作系统级别
use std::sync::{Mutex, Condvar};
let pair = Arc::new((Mutex::new(false), Condvar::new()));

// 4. 无锁数据结构
// CAS 循环实现无锁栈/队列——这在 Java 中必须用 Unsafe 类
```

**深度对比**：

| 能力 | Rust | Java | Go |
|------|------|------|-----|
| 原子操作 | ✅ `Atomic*` | ✅ `Atomic*` | ✅ `sync/atomic` |
| Memory Ordering | ✅ 5 种 ordering | ❌ 隐藏在 volatile 中 | ❌ 无 |
| compare_exchange | ✅ 硬件 CAS | ⚠️ `compareAndSet`（受限于 JVM） | ❌ 无 |
| 自旋锁 | ✅ 可以精确实现 | ⚠️ `SpinWait` 不精确 | ❌ 无 |
| 无锁数据结构 | ✅ 用户可实现 | ⚠️ 需要 `Unsafe` | ❌ 极难 |
| 线程亲和性 | ✅ 可控制 | ❌ JVM 管理 | ❌ runtime 管理 |

---

### 维度 4：系统调用与 FFI

#### Java — JNI 是噩梦

```java
// Java 调用 C 函数需要：
// 1. 写 native 方法声明
// 2. 用 javah 生成 C 头文件
// 3. 写 C 代码实现 JNI 函数
// 4. 编译成 .so/.dll
// 5. 加载库
// 6. 调用

// 更糟糕的是 Panama/FFM API（JDK 22+）仍在成熟中
```

#### Go — cgo 有开销

```go
// Go 调用 C：
// import "C"
// // #include <sys/epoll.h>
// import "C"
//
// C.epoll_create1(0)
//
// 问题：
// - cgo 调用有 ~200ns 开销（Go ↔ C 栈切换）
// - cgo 破坏了 Go 的交叉编译
// - cgo 调用不受 Go runtime 管理
// - 生产环境通常避免 cgo
```

#### Rust — 零开销 FFI

```rust
// Rust 调用 C：直接调用，零开销
extern "C" {
    fn epoll_create1(flags: i32) -> i32;
    fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut epoll_event) -> i32;
}

// 调用
unsafe {
    let epfd = epoll_create1(0);
}

// Rust 函数导出给 C 调用：
#[no_mangle]
pub extern "C" fn rust_function(x: i32) -> i32 {
    x * 2
}
// 编译后的 .a/.so 可以直接链接到 C 程序

// 内联汇编（直接在 Rust 中写汇编）
unsafe {
    let result: u64;
    core::arch::asm!(
        "rdtsc",                    // 读取时间戳计数器
        "shl rdx, 32",
        "or rax, rdx",
        out("rax") result,
        out("rdx") _,
    );
}
```

---

### 维度 5：类型系统深度

#### Java — 泛型擦除

```java
// Java 泛型在运行时被擦除
List<String> list = new ArrayList<>();
// 运行时：List list（没有 String）
// 无法 new T()
// 无法 T.class
// 无法 T[] array = new T[10]
```

#### Go — 有限泛型

```go
// Go 1.18+ 有泛型，但受限制
func Min[T constraints.Ordered](a, b T) T {
    if a < b { return a }
    return b
}
// 没有 enum + 泛型的穷尽检查
// 没有 GAT（泛型关联类型）
// 没有 const 泛型
```

#### Rust — 全功能类型系统

```rust
// 1. 泛型 + 约束
fn process<T: Read + Write>(stream: &mut T) { ... }

// 2. 关联类型
trait Container {
    type Item;  // Java/Go 没有这个
}

// 3. GAT（泛型关联类型）
trait StreamingIterator {
    type Item<'a>;  // Java/Go 完全不可能
}

// 4. const 泛型（编译期值参数）
fn process_array<const N: usize>(arr: [u8; N]) { ... }
// Java: 无法实现——数组长度不是类型参数
// Go:   无法实现——没有 const 泛型

// 5. 类型状态模式
struct Unconfigured;
struct Configured { addr: SocketAddr }

struct Server<State> {
    state: State,
}

impl Server<Unconfigured> {
    fn bind(self, addr: &str) -> Server<Configured> {
        // 状态转换：Unconfigured → Configured
        // 你不可能在 Unconfigured 状态调用 serve()——编译器阻止
        Server { state: Configured { addr: addr.parse().unwrap() } }
    }
}

impl Server<Configured> {
    fn serve(self) { ... }  // 只有 Configured 状态才能调用
}

// 6. 零大小类型（ZST）
struct Empty;  // 大小 = 0 字节
// 用于类型标记，不占内存
// Java 的空类至少 16 字节（对象头）
```

---

### 维度 6：编译期计算

#### Java — 注解处理器

```java
// Java 的编译期计算通过注解处理器实现
@AutoValue  // Lombok/AutoValue 在编译期生成代码
public abstract class Animal {
    public abstract String name();
    public abstract int age();
}
// 有限，笨重，无法做任意计算
```

#### Go — go:generate

```go
//go:generate stringer -type=Color
// 外部工具在编译前运行
// 不是语言内功能
```

#### Rust — 过程宏 + const fn

```rust
// 1. const fn：编译期函数
const fn fibonacci(n: u64) -> u64 {
    if n <= 1 { return n; }
    fibonacci(n - 1) + fibonacci(n - 2)
}

// 编译期计算
const FIB_10: u64 = fibonacci(10);  // 值在编译期确定，直接嵌入二进制
// Java 不可能——所有计算在运行时

// 2. 过程宏：编译期代码生成
#[derive(Debug, Serialize, Deserialize)]  // 编译期生成 trait 实现
struct User {
    name: String,
    age: u32,
}

// 3. 编译期类型检查
macro_rules! assert_eq_size {
    ($t:ty, $size:expr) => {
        const _: () = assert!(std::mem::size_of::<$t>() == $size);
    };
}
assert_eq_size!(Header, 64);  // 编译失败如果 Header 不是 64 字节
```

---

### 维度 7：no_std 与裸机

#### Java/Go — 不可能

```
Java/Go 都需要 runtime：
- Java 需要 JVM（几十 MB）
- Go 需要 goroutine scheduler + GC（几 MB）
- 没有 runtime = 无法运行
```

#### Rust — 可以没有标准库、没有操作系统

```rust
// #![no_std] —— 不使用标准库
// #![no_main] —— 不使用标准入口点

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// 直接写硬件寄存器
const UART0: *mut u8 = 0x1000_0000 as *mut u8;

fn print(s: &str) {
    for &byte in s.as_bytes() {
        unsafe {
            core::ptr::write_volatile(UART0, byte);
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print("Hello from bare metal!\n");
    loop {}
}
```

**这意味着 Rust 可以跑在**：STM32 微控制器、树莓派 bootloader、OS 内核、UEFI 应用、WebAssembly（无 std）。

Java/Go 在这些场景下**完全不可能**运行。

---

## 总结对比表

| API 深度维度 | Rust | Java | Go |
|-------------|------|------|-----|
| **内存控制** | ✅ 栈/堆/未初始化/自定义分配器 | ❌ GC 管理 | ❌ GC 管理 |
| **指针操作** | ✅ 4 种指针 + 指针运算 | ❌ 无指针 | ⚠️ unsafe.Pointer 受限 |
| **并发原语** | ✅ 原子 + memory ordering + CAS | ⚠️ 高层封装 | ⚠️ channel + 基础原子 |
| **FFI** | ✅ 零开销 C 调用 + 内联汇编 | ❌ JNI 开销大 | ⚠️ cgo 有开销 |
| **类型系统** | ✅ GAT + const 泛型 + ZST | ⚠️ 泛型擦除 | ⚠️ 有限泛型 |
| **编译期计算** | ✅ const fn + 过程宏 | ⚠️ 注解处理器 | ⚠️ go:generate |
| **裸机运行** | ✅ no_std | ❌ 需要 JVM | ❌ 需要 runtime |
| **内存布局** | ✅ #[repr(C/packed/align)] | ❌ JVM 控制 | ❌ 编译器控制 |
| **SIMD** | ✅ std::simd + intrinsics | ❌ 无 | ❌ 无 |
| **异步模型** | ✅ 自己控制 executor | ❌ 虚拟线程（受限于 JVM） | ✅ goroutine（受限于 runtime） |

---

## 不是"更多"，而是"更深"

```
API 数量对比：
  Java JDK:  ~4000 个公开类（最宽）
  Go std:    ~200 个包（中等）
  Rust std:  ~300 个模块（中等）

API 深度对比：
  Rust:  硬件 ← syscall ← libc ← std ← 生态（最深）
  Go:    runtime ← syscall ← std ← 生态（中等）
  Java:  JVM ← JNI ← std ← 生态（最浅的底层访问）
```

**Rust 的 API 策略**：不提供最多的 API，但提供最深的 API。从硬件寄存器到 Web 框架，同一个语言、同一套工具链。

---

## 运行

本模块为纯知识文档，无需运行代码。

相关 lab 模块：
- `raw_socket_io`：直接调用 epoll/IOCP
- `rust_pointers`：4 种指针类型对比
- `memory_management`：栈/堆/分配器
- `thread_safety`：原子操作 + memory ordering
