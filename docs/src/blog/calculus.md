# Parser Calculus: Monads, and more

## What Is a Parser?

What’s a Parser?

Ask different folks, and you’ll get different takes. Wikipedia calls a parser a software component that chews on input data and spits out a data structure. Fans of functional programming? They’ll tell you a parser is a monad. And if you’re diving into language creation, a parser is that trusty program whipped up by tools like yacc, bison, or lalrpop.

Regardless of these varying definitions, a parser always deals with syntax or grammar. The key thing is, the syntax lives in our heads, but the parser brings it to life in the real world. Parser libraries work their magic by turning our mental syntax into a runnable program.

Check out this grammar snippet (it’s in the "Parse It" style, which should be a breeze to follow, I think):

```rust
Digit -> char {
    @['0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'] => self,
}

Num -> i32 {
    digits:Digit+ => digits.into_iter().collect::<String>().parse::<i32>().unwrap(),
}

pub Expr -> i32 {
    lhs:Expr '+' rhs:Term => lhs + rhs,
    Term => self,
}

Term -> i32 {
    Num => self,
    '(' Expr ')' => self,
}
```

There are all sorts of syntax structures out there. Some are just combinations of basic elements (think terminal symbols in parsing lingo), while others refer to other rules (the non-terminal ones). Some structures zoom in on a specific part of the input and give it a name, while others just let it slide.

The concrete syntax tree shapes match the structure of the syntax perfectly. To apply the attached actions to it, we have to extract the essence of the tree in different ways depending on its structure. We keep all these differences in mind, but the parser generator has to work in a deterministic way. It follows some given rules and manages these different types while staying true to our gut feeling about the input's structure. Sounds like magic, right?

So, how do we pull this off?

## PEG and Monads

Let's try monads first. Don't be afraid of monads; just think of monads as a kind of building block that you can piece together.

```rust
type Parser<T> = impl Fn(State) -> (Result<T, Error>, State);
```

The PEG grammar that parse-it uses is structured in an inductive way:

```rust
enum Grammar {
    Just(char),           // 'a'
    Seq(Vec<Grammar>),    // A B
    Choice(Vec<Grammar>), // A | B | ..
    Option(Grammar),      // A?
    Repeat(Grammar),      // A+
}
```

(Ignore Rust's complaint about infinite sizes; we're in the land of math here.)

So, we can assign each variant a type using the parser monad:

```rust
Just: Fn(char) -> Parser<char>,
Seq: Fn(Parser<A>, Parser<B>) -> Parser<(A, B)>,
Choice: Fn(Vec<Parser<T>>) -> Parser<T>,
Option: Fn(Parser<T>) -> Parser<T>,
Repeat: Fn(Parser<T>) -> Parser<Vec<T>>,
```

And that's it. The type parameters are what we will get from the grammar. Passing it to the actions can be as simple as a monadic fmap.

```rust
Map: Fn(Parser<T>, Fn(T) -> U) -> Parser<U>,
```

So cool! Let's try it out on our expression grammar:

```rust
// Digit+
let p: Parser<char> = Digit();
let p: Parser<Vec<char>> = Repeat(p);
let p: Parser<i32> = p.map(|digits| digits.into_iter().collect::<String>().parse::<i32>().unwrap());

// '(' Expr ')'
let p1: Parser<char> = char('(');
let p2: Parser<i32> = Expr();
let p3: Parser<char> = char(')');
let p: Parser<((char, i32), char)> = Then(Then(p1, p2), p3);
// let p: Parser<i32> = p.map(|expr| expr); // <-- oops!
let p: Parser<i32> = p.map(|((_, expr), _)| expr); // ... wait a minute?
```

It works fine with the `Digit+` syntax. But as soon as we throw in the `Expr` rule, things start to go haywire. Remember how we wrote the rule: `'(' Expr ')' => self`. We just want to capture the `Expr` part and ignore the pesky parentheses. But parser monads don't get that - they dutifully record everything, even the stuff we don't care about.

Luckily, there's a workaround: we can add two more combinators to selectively ignore some elements.

```rust
ThenIgnore: Fn(Parser<A>, Parser<B>) -> Parser<A>,
IgnoreThen: Fn(Parser<A>, Parser<B>) -> Parser<B>,
```

With these, we can now write the `Expr` rule in a more sane way:

```rust
let p1: Parser<char> = char('(');
let p2: Parser<i32> = Expr();
let p3: Parser<char> = char(')');
let p: Parser<i32> = ThenIgnore(IgnoreThen(p1, p2), p3);
let p: Parser<i32> = p.map(|expr| expr);
```

But here's the catch. When should we opt for the variant that ignores something? We aim for simplicity, so we decide to ignore the parentheses. But how does the parser generator figure that out?

## Deal with captures

We can't just return everything we parse; we need to zero in on the important parts. For example, when parsing something inside parentheses like the rule `'(' expr ')'`, we only care about the `expr` part and want to ignore the parentheses. This shows our gut feeling about the input structure: non-terminals are important, while terminals aren't.

But we can't completely ignore terminals. We need to capture them somewhere; otherwise, the parser won't give us any useful output. For instance, if we write `'0' | '1' => self`, we’re capturing the digit because there's nothing else to grab.

This leads us to think of syntax structures as either "silent" or "loud." Terminals are silent, so when they appear in a sequence, their presence is overshadowed by the "loud" non-terminals, letting us ignore them.

Here's the rules about "slilent" and "loud" syntax:

- Terminals are default to be silent, while non-terminals are loud.
