# 父类泛型信息传递：Java 经典模式与 Rust 等价实现

## 设计背景与问题域

用户描述了一个在 Java 中非常优雅的设计模式：

```java
// Java：父类维护 Class<T> 字段，子类初始化时传递泛型类型
public abstract class BaseService<T> {
    protected Class<T> entityClass;

    public BaseService() {
        // 通过反射从子类的泛型签名中提取 T 的具体类型
        ParameterizedType type = (ParameterizedType) getClass().getGenericSuperclass();
        this.entityClass = (Class<T>) type.getActualTypeArguments()[0];
    }

    // 通过 entityClass 实现各种泛型操作
    public T deserialize(String json) {
        return new Gson().fromJson(json, entityClass);
    }

    public String serialize(T entity) {
        return new Gson().toJson(entity);
    }
}

public class UserService extends BaseService<User> {
    // 无需任何代码，自动获得 User 的序列化/反序列化能力
}
```

这种模式的核心价值：
1. **信息内聚**：类型信息封装在父类中，子类只需声明泛型参数
2. **能力复用**：父类基于类型信息提供通用能力（序列化、CRUD、验证等）
3. **解耦优雅**：子类与具体序列化库解耦

**问题**：Rust 如何实现这种模式？

---

## 为什么 Rust 不能直接复制这个模式

### Java 模式依赖的三个运行时能力

| 能力 | Java 实现 | Rust 状态 |
|------|----------|----------|
| 获取父类的泛型参数 | `getGenericSuperclass()` | ❌ 单态化后无泛型信息 |
| 运行时类型对象 | `Class<T>` | ⚠️ 只有 `TypeId`（无构造/方法信息） |
| 运行时反序列化 | `Gson.fromJson(json, class)` | ⚠️ 需要编译期已知类型 |

Rust 的 `TypeId` 只是一个 64 位哈希值，**不能**像 Java 的 `Class<T>` 那样用于创建实例或调用方法。

---

## Rust 的等价实现策略

### 策略一：编译期类型参数（最直接替代）

Rust 中类型参数在编译期完全确定，无需"传递"。父 struct 直接通过泛型参数获得类型信息：

```rust
use serde::{Deserialize, Serialize};

// 父 struct 持有类型参数 T，编译期自动单态化
struct BaseService<T: Entity> {
    // PhantomData 不占用空间，但让 T 在类型系统中可见
    _marker: std::marker::PhantomData<T>,
}

// trait 定义实体必须具备的能力
trait Entity: Serialize + for<'de> Deserialize<'de> + Default + 'static {
    fn type_name() -> &'static str;
}

impl<T: Entity> BaseService<T> {
    fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    // 序列化：编译期确定 T 实现了 Serialize
    fn serialize(&self, entity: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string(entity)
    }

    // 反序列化：编译期确定 T 实现了 Deserialize
    fn deserialize(&self, json: &str) -> Result<T, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct User {
    id: u64,
    name: String,
}

impl Entity for User {
    fn type_name() -> &'static str { "User" }
}

// 子 struct：只需指定类型参数
struct UserService {
    base: BaseService<User>,
}

impl UserService {
    fn new() -> Self {
        Self { base: BaseService::new() }
    }
}
```

**与 Java 对比**：
- Java：子类 `extends BaseService<User>` → 运行时父类通过反射获取 `User.class`
- Rust：子 struct `base: BaseService<User>` → 编译期直接生成 `BaseService_User` 的代码

**关键差异**：Rust 不需要"运行时获取类型"这个过程，因为类型在编译时就已经完全确定了。

---

### 策略二：显式传递序列化函数（运行时灵活性）

如果需要在运行时切换序列化策略，可以显式传递函数指针：

```rust
struct BaseService<T> {
    serialize_fn: fn(&T) -> Result<String, serde_json::Error>,
    deserialize_fn: fn(&str) -> Result<T, serde_json::Error>,
    type_name: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T> BaseService<T> {
    fn new(
        serialize_fn: fn(&T) -> Result<String, serde_json::Error>,
        deserialize_fn: fn(&str) -> Result<T, serde_json::Error>,
    ) -> Self {
        Self {
            serialize_fn,
            deserialize_fn,
            type_name: std::any::type_name::<T>(),
            _marker: std::marker::PhantomData,
        }
    }

    fn serialize(&self, entity: &T) -> Result<String, serde_json::Error> {
        (self.serialize_fn)(entity)
    }

    fn deserialize(&self, json: &str) -> Result<T, serde_json::Error> {
        (self.deserialize_fn)(json)
    }
}

// 使用：显式传入 serde 的序列化/反序列化函数
let service = BaseService::<User>::new(
    serde_json::to_string,
    serde_json::from_str,
);
```

**与 Java 对比**：
- Java：`Class<T>` 对象内部携带了类型的完整元数据（字段、方法、构造器）
- Rust：必须显式传递类型需要的具体操作函数

---

### 策略三：运行时类型注册表（最接近 Java Class 对象）

如果必须在运行时根据类型名做分发，可以用 `TypeId` + 注册表：

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;

struct TypeRegistry {
    serializers: HashMap<TypeId, Box<dyn Fn(&dyn Any) -> Result<String, String>>>,
    deserializers: HashMap<TypeId, Box<dyn Fn(&str) -> Result<Box<dyn Any>, String>>>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            serializers: HashMap::new(),
            deserializers: HashMap::new(),
        }
    }

    fn register<T: Serialize + for<'de> Deserialize<'de> + Any + 'static>(&mut self) {
        self.serializers.insert(
            TypeId::of::<T>(),
            Box::new(|any| {
                any.downcast_ref::<T>()
                    .ok_or("Type mismatch".into())
                    .and_then(|t| serde_json::to_string(t).map_err(|e| e.to_string()))
            }),
        );

        self.deserializers.insert(
            TypeId::of::<T>(),
            Box::new(|json| {
                serde_json::from_str::<T>(json)
                    .map(|t| Box::new(t) as Box<dyn Any>)
                    .map_err(|e| e.to_string())
            }),
        );
    }
}
```

**与 Java 对比**：
- Java：`Class<T>` 是编译器/运行时提供的全能类型对象
- Rust：必须手动构建类型到操作函数的映射表
- 性能：Java 反射有运行时开销，Rust 注册表是 HashMap 查找 + 动态分发

---

### 策略四：关联类型 + Trait Inheritance（Rust 风格的优雅）

Rust 中更地道的做法是使用**关联类型**而非泛型参数：

```rust
trait Service {
    type Entity: Serialize + for<'de> Deserialize<'de> + Default;

    fn serialize(&self, entity: &Self::Entity) -> Result<String, serde_json::Error> {
        serde_json::to_string(entity)
    }

    fn deserialize(&self, json: &str) -> Result<Self::Entity, serde_json::Error> {
        serde_json::from_str(json)
    }
}

struct UserService;

impl Service for UserService {
    type Entity = User;
}

// 使用：关联类型在 impl 中确定，编译期可知
let service = UserService;
let user: User = service.deserialize(r#"{"id":1,"name":"Alice"}"#)?;
```

**与 Java 对比**：
- Java：泛型参数在类声明时传递，运行时通过反射读取
- Rust：关联类型在 `impl` 时确定，编译期直接替换为具体类型
- 优势：无运行时开销，类型安全由编译器保证

---

## 设计哲学对比

| 维度 | Java 模式 | Rust 等价方案 |
|------|----------|-------------|
| 类型信息传递 | 运行时反射获取 | 编译期泛型参数 / 关联类型 |
| 序列化能力来源 | `Class<T>` 元数据 | Serde trait (Serialize/Deserialize) |
| 子类声明方式 | `extends BaseService<User>` | `base: BaseService<User>` 或关联类型 |
| 运行时灵活性 | 高（可动态加载类） | 中（需显式注册或编译期确定） |
| 类型安全 | 运行时检查（可能 ClassCastException） | 编译期保证 |
| 性能 | 反射有开销 | 零开销（单态化） |

---

## 总结

Rust 无法直接复制 Java 的"父类反射获取子类泛型"模式，因为 Rust 的泛型在编译后完全消失。但 Rust 提供了更强大的编译期抽象来达成同样的解耦效果：

1. **泛型参数 + PhantomData**：编译期自动单态化，无需运行时传递类型信息
2. **Trait Bounds**：用 `Serialize + Deserialize` 约束替代 Java 的 `Class<T>` 元数据
3. **关联类型**：Rust 风格的"每个实现者确定自己的关联类型"
4. **显式注册表**：当确实需要运行时灵活性时，手动构建 TypeId 到操作函数的映射

核心洞察：Java 的优雅来自于运行时的类型内省能力，Rust 的优雅来自于编译期的类型约束能力。两者达到同样的解耦效果，但一个在运行时、一个在编译期。
