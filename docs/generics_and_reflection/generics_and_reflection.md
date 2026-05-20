# Rust 泛型与反射：Java 式泛型反射在 Rust 中的等价实现

## 设计背景与问题域

Java 中有一种经典的 OOP 解耦模式：父类声明泛型参数 `T`，子类实例化时指定具体类型，父类通过反射在运行时获取 `T` 的实际类型，进而创建对象或调用类型特定的方法。

```java
// Java：父类通过反射获取子类指定的泛型参数
public abstract class BaseDao<T> {
    private Class<T> entityClass;

    public BaseDao() {
        // 通过反射获取泛型参数 T 的实际类型
        ParameterizedType type = (ParameterizedType) getClass().getGenericSuperclass();
        this.entityClass = (Class<T>) type.getActualTypeArguments()[0];
    }

    public T createInstance() throws Exception {
        return entityClass.getDeclaredConstructor().newInstance();
    }
}

public class UserDao extends BaseDao<User> {
    // UserDao 无需任何代码，自动获得操作 User 的能力
}
```

**用户的问题**：Rust 是否支持这种模式？

**简短的回答**：**不支持，且设计上不可能支持**。但 Rust 提供了完全不同的抽象工具来实现同等甚至更强的解耦效果。

---

## 为什么 Rust 不可能支持 Java 式的泛型反射

### 根本原因：单态化（Monomorphization） vs 类型擦除（Type Erasure）

| 维度 | Rust 泛型 | Java 泛型 |
|------|----------|----------|
| 编译策略 | 单态化（每个具体类型生成一份代码） | 类型擦除（编译后变为 Object + 强制转换） |
| 运行时信息 | **无**泛型参数信息 | 保留在 class 文件的 Signature 属性中 |
| 反射能力 | 无 Java 式反射 | `getGenericSuperclass()` 可获取泛型参数 |
| 性能 | 零运行时开销 | 类型转换和装箱开销 |

```rust
// Rust：Vec<i32> 和 Vec<String> 是完全不同的类型
// 编译后不存在 "Vec<T>"，只存在 Vec_i32 和 Vec_String
let v1: Vec<i32> = vec![1, 2, 3];
let v2: Vec<String> = vec!["a".into(), "b".into()];

// 运行时没有 "T" 的概念，无法通过任何 API 获取 "这个 Vec 的 T 是什么"
```

```java
// Java：List<Integer> 和 List<String> 运行时都是 List（类型擦除）
// 但通过反射可以获取泛型参数信息
List<Integer> list = new ArrayList<>();
Type type = list.getClass().getGenericSuperclass();
// type 包含 Integer 的信息
```

**Rust 的设计权衡**：用编译期信息换取运行时性能。泛型参数在编译后完全消失，因此运行时不可能"反推"泛型参数是什么。

### Rust 没有 Java 式反射

Rust 的"反射"极其有限：

| 能力 | Java | Rust |
|------|------|------|
| 获取类的字段列表 | `Class.getDeclaredFields()` | ❌ 不支持 |
| 获取方法列表 | `Class.getMethods()` | ❌ 不支持 |
| 动态创建对象 | `Class.newInstance()` | ❌ 不支持 |
| 调用任意方法 | `Method.invoke()` | ❌ 不支持 |
| 获取泛型参数 | `getActualTypeArguments()` | ❌ 不支持 |
| 运行时类型识别 | `instanceof` | `Any::downcast_ref`（有限） |
| 类型唯一标识 | `Class` 对象 | `TypeId` |

---

## Rust 的替代方案：从"运行时反射"到"编译期多态"

### 方案一：显式传递工厂函数（最直接替代）

既然无法在运行时"反推"类型，就**在构造时显式传入创建逻辑**：

```rust
// Rust：用泛型 + 闭包实现 Java BaseDao 的等价功能
struct BaseDao<T> {
    create_fn: fn() -> T,  // 显式传递构造函数
}

impl<T> BaseDao<T> {
    fn new(create_fn: fn() -> T) -> Self {
        Self { create_fn }
    }

    fn create_instance(&self) -> T {
        (self.create_fn)()
    }
}

struct User {
    name: String,
}

impl User {
    fn new() -> Self {
        Self { name: "default".into() }
    }
}

// 使用：显式指定类型和工厂函数
let user_dao = BaseDao::new(User::new);
let user = user_dao.create_instance();
```

**与 Java 对比**：
- Java：隐式通过反射获取类型信息
- Rust：显式传递构造函数，编译期确定，零运行时开销

### 方案二：Trait 对象（运行时多态）

如果需要在运行时处理不同类型的对象，使用 `dyn Trait`：

```rust
trait Entity: std::fmt::Debug {
    fn entity_type(&self) -> &'static str;
}

#[derive(Debug)]
struct User { name: String }

impl Entity for User {
    fn entity_type(&self) -> &'static str { "User" }
}

#[derive(Debug)]
struct Product { id: u64 }

impl Entity for Product {
    fn entity_type(&self) -> &'static str { "Product" }
}

// 运行时多态：Vec 中存放不同类型的对象
fn process_entities(entities: Vec<Box<dyn Entity>>) {
    for e in entities {
        println!("Processing: {:?} (type: {})", e, e.entity_type());
    }
}
```

**关键差异**：
- Java 泛型反射：运行时知道 `T` 是 `User`，可以 `new User()`
- Rust trait 对象：运行时知道对象实现了 `Entity`，但不知道具体类型，除非 downcast

### 方案三：`Any` trait + downcast（有限的运行时类型识别）

Rust 提供 `std::any::Any` trait 实现有限的运行时类型识别：

```rust
use std::any::{Any, TypeId};

fn inspect_any(value: &dyn Any) {
    println!("TypeId: {:?}", value.type_id());

    if let Some(user) = value.downcast_ref::<User>() {
        println!("It's a User: {:?}", user);
    } else if let Some(product) = value.downcast_ref::<Product>() {
        println!("It's a Product: {:?}", product);
    } else {
        println!("Unknown type");
    }
}

fn main() {
    let user = User { name: "Alice".into() };
    inspect_any(&user);
}
```

**限制**：
- 只能 downcast 到**具体类型**，不能 downcast 到泛型参数 `T`
- `TypeId` 是编译期生成的唯一标识，运行时只能判断"是不是这个类型"，不能获取类型信息

### 方案四：PhantomData — 在类型系统中携带信息

Rust 用 `PhantomData` 在类型系统中编码信息，编译期利用，运行时零成本：

```rust
use std::marker::PhantomData;

// 用 PhantomData<T> 在类型系统中携带 T 的信息
struct TypeTag<T> {
    _marker: PhantomData<T>,
}

impl<T: Default + 'static> TypeTag<T> {
    fn new() -> Self {
        Self { _marker: PhantomData }
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn create_default(&self) -> T {
        T::default()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

// 使用
let tag: TypeTag<String> = TypeTag::new();
println!("Type: {}", tag.type_name()); // "alloc::string::String"
let s = tag.create_default(); // 创建 String::default()
```

**设计洞察**：PhantomData 不占用运行时空间，但让类型参数 `T` 在编译期"可见"。这是一种**编译期反射**。

### 方案五：编译期 Trait Bounds（Rust 的"真解"）

Java 用"泛型 + 反射"解决的问题，在 Rust 中通常用**更强的类型约束**直接消除：

```rust
// Java 思路：BaseDao<T> 通过反射知道 T 有什么方法
// Rust 思路：直接用 trait bounds 要求 T 必须实现某些能力

trait Dao<T: Entity + Default> {
    fn create(&self) -> T {
        T::default()  // 编译期确定 T 实现了 Default
    }

    fn save(&self, entity: &T) {
        entity.save_to_db();  // 编译期确定 T 实现了 Entity
    }
}

trait Entity: Default + std::fmt::Debug {
    fn save_to_db(&self) {
        println!("Saving {:?} to database", self);
    }
}

#[derive(Debug, Default)]
struct User { name: String }

impl Entity for User {}

// UserDao 不需要"知道" User 的类型信息——编译器已经保证了 User 实现了 Entity + Default
struct UserDao;
impl Dao<User> for UserDao {}
```

**关键差异**：
- Java：运行时通过反射检查类型能力，可能运行时失败
- Rust：编译期通过 trait bounds 强制类型必须具有某些能力，**不可能运行时失败**

---

## 设计哲学对比

| 维度 | Java（泛型 + 反射） | Rust（泛型 + Trait） |
|------|-------------------|-------------------|
| 解耦方式 | 运行时类型检查 | 编译期类型约束 |
| 错误发现 | 运行期（反射调用失败） | 编译期（trait 不满足） |
| 性能 | 反射有开销 | 零开销（单态化） |
| 灵活性 | 高（运行时决定行为） | 中（编译期决定行为） |
| 安全性 | 低（运行时可能 ClassCastException） | 高（编译期保证） |

---

## 延伸阅读

- [父类泛型信息传递：Java 经典模式与 Rust 等价实现](generic_type_passing.md)

## 运行示例

```bash
cargo run -p generics_and_reflection
```
