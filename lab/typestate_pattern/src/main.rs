// 类型状态模式：用类型系统编码状态机
//
// 设计意图：
// - 传统 OOP 在运行时检查状态，Rust 在编译期就阻止非法状态转换
// - PhantomData<State> 不占用运行时空间，零成本抽象
// - 状态转换通过消耗旧值、返回新值实现（所有权转移）

use std::marker::PhantomData;

// === 状态类型 ===
// 这些类型没有数据，仅在类型系统中存在
struct Idle;
struct Running {
    speed: u32,
}
struct Stopped;

// === 类型状态机器 ===
// State 泛型参数将状态编码到类型中
struct Motor<State> {
    _state: PhantomData<State>,
}

// Idle 状态可执行的操作
impl Motor<Idle> {
    fn new() -> Self {
        Motor { _state: PhantomData }
    }

    fn start(self, speed: u32) -> Motor<Running> {
        println!("Motor starting at {} RPM", speed);
        Motor { _state: PhantomData }
    }
}

// Running 状态可执行的操作
impl Motor<Running> {
    fn stop(self) -> Motor<Stopped> {
        println!("Motor stopping");
        Motor { _state: PhantomData }
    }

    fn get_speed(&self) -> u32 {
        // 由于 Running 是私有字段构造的，这里我们知道 speed
        // 实际场景中 Running 结构体可以包含真实数据
        100 // 简化示例
    }
}

// Stopped 状态可执行的操作
impl Motor<Stopped> {
    fn restart(self, speed: u32) -> Motor<Running> {
        println!("Motor restarting at {} RPM", speed);
        Motor { _state: PhantomData }
    }
}

fn main() {
    // 1. 创建 Idle 状态的电机
    let motor = Motor::new();

    // motor.stop(); // 编译错误！Idle 状态的电机不能 stop

    // 2. 启动电机，状态变为 Running
    let motor = motor.start(1500);
    println!("Motor is running");

    // motor.start(2000); // 编译错误！Running 状态的电机不能 start

    // 3. 停止电机，状态变为 Stopped
    let motor = motor.stop();

    // motor.stop(); // 编译错误！Stopped 状态的电机不能 stop

    // 4. 重新启动
    let motor = motor.restart(2000);

    // 5. 最终停止
    let _motor = motor.stop();

    // 编译器保证：不可能在 Idle 状态调用 stop，不可能在 Running 状态调用 start
    // 这些错误在编译期就被捕获，无需运行时检查，零运行时开销
}
