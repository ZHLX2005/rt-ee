// Rust 泛型与反射：Java 式泛型反射的替代方案
//
// 设计意图：
// - 展示 Rust 为什么不支持 Java 式的泛型反射
// - 展示 Rust 提供的替代方案实现同等解耦效果
// - 对比编译期多态 vs 运行时反射的设计哲学

use std::any::{Any, TypeId};
use std::marker::PhantomData;

// === 方案一：显式传递工厂函数 ===
// 替代 Java 的 "newInstance()" 反射调用

struct BaseDao<T> {
    create_fn: fn() -> T,
}

impl<T> BaseDao<T> {
    fn new(create_fn: fn() -> T) -> Self {
        Self { create_fn }
    }

    fn create_instance(&self) -> T {
        (self.create_fn)()
    }
}

#[derive(Debug, Default)]
struct User {
    name: String,
}

impl User {
    fn new() -> Self {
        Self {
            name: "default_user".into(),
        }
    }
}

#[derive(Debug, Default)]
struct Product {
    id: u64,
}

impl Product {
    fn new() -> Self {
        Self { id: 0 }
    }
}

fn demo_factory_pattern() {
    println!("=== 方案一：显式工厂函数 ===");
    let user_dao = BaseDao::new(User::new);
    let user = user_dao.create_instance();
    println!("Created user: {:?}", user);

    let product_dao = BaseDao::new(Product::new);
    let product = product_dao.create_instance();
    println!("Created product: {:?}", product);
}

// === 方案二：Trait 对象（运行时多态） ===
// 当需要在运行时处理异构类型集合时使用

trait Entity: std::fmt::Debug {
    fn entity_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}

impl Entity for User {
    fn entity_type(&self) -> &'static str {
        "User"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Entity for Product {
    fn entity_type(&self) -> &'static str {
        "Product"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn process_entities(entities: Vec<Box<dyn Entity>>) {
    println!("\n=== 方案二：Trait 对象（运行时多态）===");
    for e in entities {
        println!("Processing: {:?} (type: {})", e, e.entity_type());

        // 尝试 downcast 到具体类型
        if let Some(user) = e.as_any().downcast_ref::<User>() {
            println!("  Downcasted to User: name={}", user.name);
        } else if let Some(product) = e.as_any().downcast_ref::<Product>() {
            println!("  Downcasted to Product: id={}", product.id);
        }
    }
}

// === 方案三：Any trait + downcast（有限运行时识别） ===

fn inspect_any(value: &dyn Any) {
    println!("\n=== 方案三：Any trait + downcast ===");
    println!("TypeId: {:?}", value.type_id());

    if let Some(user) = value.downcast_ref::<User>() {
        println!("It's a User: {:?}", user);
    } else if let Some(product) = value.downcast_ref::<Product>() {
        println!("It's a Product: {:?}", product);
    } else {
        println!("Unknown type");
    }
}

// === 方案四：PhantomData — 编译期类型标签 ===
// 零运行时成本，在类型系统中携带类型信息

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

fn demo_phantom_data() {
    println!("\n=== 方案四：PhantomData（编译期类型标签）===");

    let user_tag: TypeTag<User> = TypeTag::new();
    println!("Type name: {}", user_tag.type_name());
    println!("TypeId: {:?}", user_tag.type_id());
    let user = user_tag.create_default();
    println!("Default instance: {:?}", user);

    // PhantomData 不占用运行时空间
    println!(
        "Size of TypeTag<User>: {} bytes (same as empty struct)",
        std::mem::size_of::<TypeTag<User>>()
    );
}

// === 方案五：编译期 Trait Bounds（Rust 的"真解"） ===
// Java 用"泛型+反射"解决的问题，Rust 用更强的类型约束直接消除

trait EntityOps: Default + std::fmt::Debug + 'static {
    fn save_to_db(&self) {
        println!("Saving {:?} to database", self);
    }
}

impl EntityOps for User {}
impl EntityOps for Product {}

// Dao 通过 trait bounds 要求 T 必须具备的能力
// 编译期保证：任何不满足条件的类型都无法通过编译
trait Dao<T: EntityOps> {
    fn create(&self) -> T {
        T::default()
    }

    fn save(&self, entity: &T) {
        entity.save_to_db();
    }
}

struct UserDao;
impl Dao<User> for UserDao {}

struct ProductDao;
impl Dao<Product> for ProductDao {}

fn demo_trait_bounds() {
    println!("\n=== 方案五：编译期 Trait Bounds ===");

    let user_dao = UserDao;
    let user = user_dao.create();
    println!("Created via trait bound: {:?}", user);
    user_dao.save(&user);

    let product_dao = ProductDao;
    let product = product_dao.create();
    println!("Created via trait bound: {:?}", product);
    product_dao.save(&product);
}

// === 运行时类型注册表（高级模式） ===
// 用 TypeId 作为 key，在运行时维护类型到处理函数的映射

use std::collections::HashMap;

struct TypeRegistry {
    handlers: HashMap<TypeId, Box<dyn Fn(&dyn Any) -> String>>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    fn register<T: Any + std::fmt::Debug>(&mut self) {
        self.handlers.insert(
            TypeId::of::<T>(),
            Box::new(|any| {
                if let Some(t) = any.downcast_ref::<T>() {
                    format!("{:?}", t)
                } else {
                    "Type mismatch".into()
                }
            }),
        );
    }

    fn handle(&self, value: &dyn Any) -> Option<String> {
        self.handlers
            .get(&value.type_id())
            .map(|handler| handler(value))
    }
}

fn demo_type_registry() {
    println!("\n=== 高级模式：运行时类型注册表 ===");

    let mut registry = TypeRegistry::new();
    registry.register::<User>();
    registry.register::<Product>();

    let user = User {
        name: "Alice".into(),
    };
    let product = Product { id: 42 };

    println!("Handle user: {:?}", registry.handle(&user));
    println!("Handle product: {:?}", registry.handle(&product));
}

// === 方案六：父类泛型信息传递（Java 经典模式的 Rust 等价实现） ===
// 用户描述的模式：父类维护类型信息，子类初始化时"传递"泛型类型，
// 父类基于类型信息提供通用能力（序列化、CRUD 等）

use serde::{Deserialize, Serialize};

// Trait 定义实体必须具备的能力（替代 Java 的 Class<T>）
trait SerializableEntity: Serialize + for<'de> Deserialize<'de> + Default + 'static {
    fn type_name() -> &'static str;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct Person {
    id: u64,
    name: String,
}

impl SerializableEntity for Person {
    fn type_name() -> &'static str { "Person" }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct Order {
    order_id: String,
    amount: f64,
}

impl SerializableEntity for Order {
    fn type_name() -> &'static str { "Order" }
}

// "父类"：通过泛型参数直接获得类型信息，编译期单态化
struct BaseService<T: SerializableEntity> {
    // PhantomData 让 T 在类型系统中可见，运行时零成本
    _marker: PhantomData<T>,
}

impl<T: SerializableEntity> BaseService<T> {
    fn new() -> Self {
        Self { _marker: PhantomData }
    }

    // 编译期确定 T 实现了 Serialize
    fn serialize(&self, entity: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string(entity)
    }

    // 编译期确定 T 实现了 Deserialize
    fn deserialize(&self, json: &str) -> Result<T, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn create_default(&self) -> T {
        T::default()
    }
}

// "子类"：只需指定类型参数
struct PersonService {
    base: BaseService<Person>,
}

impl PersonService {
    fn new() -> Self {
        Self { base: BaseService::new() }
    }

    // 委托给父类提供的通用能力
    fn serialize(&self, entity: &Person) -> Result<String, serde_json::Error> {
        self.base.serialize(entity)
    }

    fn deserialize(&self, json: &str) -> Result<Person, serde_json::Error> {
        self.base.deserialize(json)
    }
}

struct OrderService {
    base: BaseService<Order>,
}

impl OrderService {
    fn new() -> Self {
        Self { base: BaseService::new() }
    }
}

fn demo_generic_type_passing() {
    println!("\n=== 方案六：父类泛型信息传递（Java 经典模式等价实现）===");

    // PersonService 自动获得 Person 的序列化能力
    let person_service = PersonService::new();
    let person = Person { id: 1, name: "Alice".into() };

    let json = person_service.serialize(&person).unwrap();
    println!("Serialized Person: {}", json);

    let restored: Person = person_service.deserialize(&json).unwrap();
    println!("Deserialized Person: {:?}", restored);

    // OrderService 自动获得 Order 的序列化能力
    let order_service = OrderService::new();
    let order = Order { order_id: "ORD-001".into(), amount: 199.99 };

    let json = order_service.base.serialize(&order).unwrap();
    println!("Serialized Order: {}", json);

    let restored: Order = order_service.base.deserialize(&json).unwrap();
    println!("Deserialized Order: {:?}", restored);

    // 展示：PhantomData 不占用空间
    println!(
        "Size of BaseService<Person>: {} bytes",
        std::mem::size_of::<BaseService<Person>>()
    );
}

// === 方案七：关联类型（Rust 风格的地道实现） ===

trait Service {
    type Entity: Serialize + for<'de> Deserialize<'de> + Default + Clone;

    fn serialize(&self, entity: &Self::Entity) -> Result<String, serde_json::Error> {
        serde_json::to_string(entity)
    }

    fn deserialize(&self, json: &str) -> Result<Self::Entity, serde_json::Error> {
        serde_json::from_str(json)
    }
}

struct PersonServiceV2;

impl Service for PersonServiceV2 {
    type Entity = Person;
}

fn demo_associated_types() {
    println!("\n=== 方案七：关联类型（Rust 地道实现）===");

    let service = PersonServiceV2;
    let person = Person { id: 2, name: "Bob".into() };

    let json = service.serialize(&person).unwrap();
    println!("Serialized: {}", json);

    let restored = service.deserialize(&json).unwrap();
    println!("Deserialized: {:?}", restored);
}

fn main() {
    demo_factory_pattern();

    process_entities(vec![
        Box::new(User {
            name: "Bob".into(),
        }),
        Box::new(Product { id: 100 }),
    ]);

    inspect_any(&User {
        name: "Charlie".into(),
    });

    demo_phantom_data();

    demo_trait_bounds();

    demo_type_registry();

    demo_generic_type_passing();

    demo_associated_types();

    println!("\n=== 关键洞察 ===");
    println!("Rust 不支持 Java 式的泛型反射，因为泛型在编译后完全消失（单态化）。");
    println!("但 Rust 提供了更强大的替代方案：");
    println!("  1. 显式工厂函数 —— 编译期确定，零开销");
    println!("  2. Trait 对象 —— 运行时多态，显式分发");
    println!("  3. Any + downcast —— 有限的运行时类型识别");
    println!("  4. PhantomData —— 编译期类型标签，零成本");
    println!("  5. Trait Bounds —— 编译期强制约束，比反射更安全");
    println!("  6. 泛型参数 + PhantomData —— 父类自动获得子类类型信息");
    println!("  7. 关联类型 —— Rust 风格的地道类型传递方式");
}
