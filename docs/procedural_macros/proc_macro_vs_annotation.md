# Rust 过程宏 vs Java 注解处理器：AST 操作深度对比

## 设计背景与问题域

Java 程序员熟悉注解（`@Entity`、`@Autowired`、`@Override`）和注解处理器（Annotation Processor）。Rust 程序员使用过程宏（`#[derive(...)]`、`#[command]`）。两者都是编译期元编程，但**操作的对象、能力和限制完全不同**。

理解这些差异的关键问题：

1. **Rust 过程宏操作的是 Token Stream，Java 注解处理器操作的是 Element 模型。这两种抽象层级有什么本质区别？**
2. **为什么 Rust 可以修改/替换被修饰的代码，而 Java 只能读取并生成新文件？**
3. **Java 需要 AspectJ/ByteBuddy/反射来实现的功能，Rust 过程宏如何在编译期直接完成？**

---

## 核心架构对比

### Rust 过程宏：编译期代码变换器

```
源代码 Token Stream
    ↓
过程宏函数（Rust 代码在编译期执行）
    ↓ 解析（syn）、变换、生成（quote）
新的 Token Stream
    ↓
类型检查、代码生成
```

### Java 注解处理器：编译期代码生成器

```
源代码 AST
    ↓
编译器第一轮：解析符号表（Element 模型）
    ↓
注解处理器（Java 代码在编译期执行）
    ↓ 读取 Element，生成新 .java 文件
新的源代码文件
    ↓
编译器后续轮次：重新解析（如果生成新注解）
    ↓
字节码生成
```

---

## AST 表示的抽象层级

### Rust：操作具体语法树

Rust 过程宏接收的是**原始 Token Stream**（标识符、标点、字面量的扁平序列）。`syn` 库将其解析为强类型的 AST 节点：

```rust
use syn::{DeriveInput, Data, Fields, Type, Ident};

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    // 解析为强类型 AST
    let ast: DeriveInput = syn::parse(input).unwrap();

    // 直接操作语法节点：struct 名称、字段列表、类型
    let struct_name = ast.ident;              // Ident
    let fields = match ast.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,  // Punctuated<Field, Comma>
            _ => panic!("only named fields"),
        },
        _ => panic!("only structs"),
    };

    // 可以构造任意的 Rust 代码：struct、impl、fn、match...
    let generated = quote! {
        pub struct UserBuilder { /* ... */ }
        impl UserBuilder { /* ... */ }
    };

    generated.into()
}
```

**关键特征**：
- 操作的是**语法层面的具体结构**（struct 有这几个字段，类型是 String）
- 可以生成任何合法的 Rust 语法
- 生成的代码与原代码在**同一个编译单元**中

### Java：操作类型系统的符号

Java 注解处理器通过 `javax.lang.model` 包操作编译期的**类型系统符号**：

```java
@SupportedAnnotationTypes("com.example.Builder")
public class BuilderProcessor extends AbstractProcessor {
    @Override
    public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
        for (Element elem : roundEnv.getElementsAnnotatedWith(Builder.class)) {
            // Element 是类型系统的抽象，不是具体语法
            TypeElement classElem = (TypeElement) elem;
            String className = classElem.getSimpleName().toString();

            // 获取字段：通过 Enclosed Elements
            List<VariableElement> fields = new ArrayList<>();
            for (Element enclosed : classElem.getEnclosedElements()) {
                if (enclosed.getKind() == ElementKind.FIELD) {
                    fields.add((VariableElement) enclosed);
                }
            }

            // 生成新的源文件（不能修改原文件！）
            writeSourceFile(className + "Builder", generateBuilderCode(classElem, fields));
        }
        return true;
    }
}
```

**关键特征**：
- 操作的是**类型系统的抽象表示**（TypeElement、VariableElement、TypeMirror）
- 只能读取被注解元素的信息，**不能修改它**
- 输出必须是**新的 Java 源文件**

---

## 核心能力差异：修改 vs 生成

### Rust 可以替换被修饰的代码

Rust 的属性宏接收被修饰 item 的**完整 AST**，可以输出**完全不同的 AST** 来替换它：

```rust
// 输入：
#[trace_function]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// 宏输出（替换原函数）：
fn add(a: i32, b: i32) -> i32 {
    println!("[TRACE] Entering: add");
    let __result = {
        a + b  // 原函数体被包裹
    };
    println!("[TRACE] Exiting: add");
    __result
}
```

**这是 Rust 独有的能力**：过程宏可以**解构并重组**被修饰的代码。

### Java 只能生成新代码

Java 注解处理器**没有修改已有代码的能力**。要实现类似功能，必须借助其他技术：

| 目标 | Java 方案 | 限制 |
|------|----------|------|
| 给方法加日志 | AspectJ / ByteBuddy（字节码操作） | 不是注解处理器，编译后处理 |
| 给方法加日志 | 生成子类重写方法 | 需要虚函数调用开销 |
| 给方法加日志 | Spring AOP（动态代理） | 运行时开销，接口限制 |
| 自动生成 Builder | 注解处理器生成新类 | 必须新建文件 |

**Java 注解处理器的天花板**：读取信息 → 生成新文件。不能修改、不能包裹、不能替换。

---

## 编译轮次模型

### Rust：单轮次展开

```
源代码
    ↓
宏展开（所有过程宏并行/顺序执行）
    ↓
类型检查
    ↓
代码生成
```

Rust 过程宏在**类型检查之前**单轮次展开。宏之间不能"看到"彼此生成的注解（因为 Rust 没有注解的概念，只有属性）。

### Java：多轮次处理

```
Round 1: 解析 → 发现注解 → 处理器执行 → 生成新文件
    ↓（如果生成了带注解的新文件）
Round 2: 重新解析 → 发现新注解 → 处理器执行 → 生成新文件
    ↓
...（直到没有新文件生成）
```

Java 的多轮次设计允许处理器 A 生成带注解的代码，触发处理器 B 在下一轮执行。这提供了组合能力，但也增加了复杂性。

**Rust 的替代方案**：过程宏 crate 可以依赖其他过程宏 crate（通过 Cargo 依赖），在编译期顺序执行。

---

## Hygiene：标识符作用域隔离

### Rust：编译器自动保证 Hygiene

Rust 过程宏生成的标识符（变量名、标签）**不会与使用者代码冲突**，除非显式使用 `Span::call_site()`：

```rust
// 宏内部生成的变量 __result
let __result = #fn_block;

// 即使用户代码也有 __result，也不会冲突
// 因为宏生成的标识符带有宏的 hygiene 上下文
```

### Java：无 Hygiene 机制

Java 注解处理器生成的代码必须**手动避免命名冲突**：

```java
// 处理器生成代码时，必须使用不太可能与用户代码冲突的名称
// 常见做法：使用前缀或 UUID
String fieldName = "__generated_builder_" + originalFieldName;
```

这是 Java 代码生成器常见的 bug 来源。

---

## 错误报告位置

### Rust：在宏调用处报告

```rust
#[derive(Builder)]
struct User {
    name: String,
    // 如果宏展开生成的代码有类型错误，
    // 编译器会将错误定位到 #[derive(Builder)] 这一行
}
```

Rust 编译器维护**Span 信息**，知道每个生成的 Token 来自源代码的哪个位置。

### Java：在生成的代码处报告

Java 编译器对生成的代码报告错误时，错误信息指向**生成的源文件**（如 `UserBuilder.java`），而不是原始注解位置。开发者需要在生成的文件中找错误，体验较差。

---

## 实际案例：Builder 模式生成的完整对比

### 输入代码

**Rust**：
```rust
#[derive(Builder)]
struct User {
    name: String,
    age: u32,
}
```

**Java**：
```java
@Builder
public class User {
    private String name;
    private int age;
}
```

### 处理逻辑对比

| 步骤 | Rust 过程宏 | Java 注解处理器 |
|------|-----------|---------------|
| 输入 | `TokenStream`（原始 Token） | `TypeElement`（类型符号） |
| 字段遍历 | `fields.iter()` 遍历 AST 节点 | `getEnclosedElements()` 遍历符号 |
| 类型获取 | `field.ty` 直接是语法节点 | `field.asType()` 返回 `TypeMirror` |
| 输出方式 | 返回 `TokenStream`（同文件） | `writeSourceFile()` 写新文件 |
| 输出位置 | 与 `User` 同编译单元 | `UserBuilder.java` 新文件 |
| 类型检查 | 生成代码立即参与类型检查 | 新文件需要额外编译轮次 |

### 输出代码对比

**Rust 输出**（编译期嵌入）：
```rust
// 与 User 在同一个编译单元
pub struct UserBuilder {
    name: Option<String>,
    age: Option<u32>,
}

impl UserBuilder {
    pub fn name(mut self, value: String) -> Self { ... }
    pub fn age(mut self, value: u32) -> Self { ... }
    pub fn build(self) -> User { ... }
}
```

**Java 输出**（生成新文件）：
```java
// 独立的 UserBuilder.java 文件
public class UserBuilder {
    private String name;
    private int age;

    public UserBuilder name(String name) {
        this.name = name;
        return this;
    }

    public User build() {
        User user = new User();
        user.name = this.name;
        user.age = this.age;
        return user;
    }
}
```

---

## 能力矩阵总对比

| 能力 | Rust 过程宏 | Java 注解处理器 | Java + AspectJ |
|------|-----------|---------------|--------------|
| 读取 AST | ✅ Token Stream | ✅ Element 模型 | ❌ 运行时 |
| 生成新代码 | ✅ | ✅ | ✅ |
| 修改已有代码 | ✅ 替换 | ❌ 不可修改 | ✅ 字节码织入 |
| 包装函数体 | ✅ 属性宏 | ❌ | ✅ `around` advice |
|  hygiene  | ✅ 编译器保证 | ❌ 手动处理 | N/A |
| 错误定位 | ✅ 指向调用处 | ⚠️ 指向生成文件 | N/A |
| 运行时成本 | 零 | 零 | 有（代理/织入） |
| 类型安全 | ✅ 生成后类型检查 | ✅ 生成后类型检查 | ⚠️ 运行时检查 |

---

## 为什么 Java 不直接支持代码修改？

Java 注解处理器的设计受限于 **JSR 269** 的约束：

1. **增量编译兼容性**：如果处理器可以修改已有文件，增量编译（只编译变更文件）会失效
2. **IDE 支持**：Eclipse/IntelliJ 的增量编译架构假设源文件不会被修改
3. **编译器架构**：javac 的编译轮次模型基于"发现新文件"，而非"替换旧文件"

Rust 过程宏之所以可以修改代码，是因为：

1. **编译单元模型**：每个 crate 是独立的编译单元，宏展开在 crate 内完成
2. **Token Stream 抽象**：输入输出都是 Token Stream，编译器不区分"原始"和"生成"的代码
3. **无增量编译负担**：Rust 的增量编译在宏展开之后进行，基于 HIR/MIR 而非源代码

---

## 总结

Rust 过程宏和 Java 注解处理器代表了两种不同的编译期元编程哲学：

- **Rust**：**代码变换器**。操作具体语法，可以替换、包裹、修改被修饰的代码。能力是泛化的，但责任也更大（宏作者需要保证生成的代码合法）。

- **Java**：**代码生成器**。操作类型符号，只能读取信息并生成新文件。能力受限但更保守安全，不会破坏已有代码。

Java 开发者想要 Rust 过程宏的"修改能力"时，通常需要引入 **AspectJ**（编译期字节码织入）或 **ByteBuddy**（运行时字节码操作）——这些是独立于注解处理器的额外工具链。而在 Rust 中，这一切都是语言内置的宏系统的标准能力。
