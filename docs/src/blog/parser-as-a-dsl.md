# Parser as a DSL

Askar Safin's [blog post](https://safinaskar.writeas.com/this-is-why-you-should-never-use-parser-combinators-and-peg)

When using parser combinators, the syntax noise can be overwhelming. It explodes in complexity and quickly becomes unreadable. 

A parser is a set of rules and their corresponding actions. 

Deal with captures

We cannot return everything we parse. We must focus on the important parts. For example, when parsing something inside a parenthesis, such as the rule `'(' expr ')'`, we only care about the `expr` part, and the parentheses are intended to be ignored. This reveals our intuition about the structure of the input: non-terminals are important, terminals are not.

But we can't just ignore terminals. We need to capture them somewhere, too, otherwise we cannot get anything useful out of the parser. If we write `'0' | '1' => self`, our intuition is that we are indeed capturing the digit, because there is nothing else to capture. 

This inspires us to make some syntax structures "slient", and others "loud". Terminals are silent, so when they are in a sequence, their sound is drowned out by the "loud" non-terminals, thus we can ignore them.

Here's the rules about "slilent" and "loud" syntax:

- Terminals are default to be silent, while non-terminals are loud.
- 

`(head, tail)` represents a sequence