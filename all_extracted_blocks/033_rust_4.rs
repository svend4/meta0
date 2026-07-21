// Кусок реального кода ядра компилятора на Rust
enum AstNode {
    BindState { target_store: String },
    GetProperty { property: String },
    SafeMathSub,
    ConditionalBranch { on_success: Box<AstNode>, on_fail: Box<AstNode> },
    ForwardExit { status: u16 },
}