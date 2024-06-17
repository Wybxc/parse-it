# Parser as a DSL: The Story of "Parse It"

## Combinators vs. PEG

## The Problem with Parser Combinators

Askar Safin's [blog post](https://safinaskar.writeas.com/this-is-why-you-should-never-use-parser-combinators-and-peg)

Parsing can be a complex and intricate task, especially when choosing the right tools and libraries for the job. Among the various approaches to parsing, parser combinators and Parsing Expression Grammars (PEG) are popular but come with significant drawbacks. Askar Safin's insightful blog post sheds light on why parser combinators and PEG might not be the best choices for your parsing needs.

Parser combinators offer a powerful and flexible way to build parsers by combining simple parsing functions. However, this flexibility comes at a cost. The main issue with parser combinators is the overwhelming syntax noise that can quickly make your code explode in complexity and become unreadable.

When using parser combinators, each parsing rule is defined as a combination of smaller parsers. This can lead to deeply nested and convoluted code, making it hard to understand and maintain. As the complexity of the grammar increases, so does the noise in the syntax, which can be daunting for developers.

## What Is a Parser?

At its core, a parser is a set of rules and their corresponding actions. These rules define how to interpret and transform input data into a structured format that can be easily processed by a computer program. The choice of parser and parsing strategy can significantly impact the efficiency and maintainability of your code.

## Deal with captures

We cannot return everything we parse; we must focus on the important parts. For example, when parsing something inside a parenthesis, such as the rule `'(' expr ')'`, we only care about the `expr` part, and the parentheses are intended to be ignored. This reveals our intuition about the structure of the input: non-terminals are important, while terminals are not.

However, we can't entirely ignore terminals. We need to capture them somewhere; otherwise, the parser won't produce any useful output. For instance, if we write `'0' | '1' => self`, our intuition is that we are capturing the digit because there is nothing else to capture.

This inspires us to categorize syntax structures as "silent" or "loud". Terminals are silent, so when they are in a sequence, their presence is overshadowed by the "loud" non-terminals, allowing us to ignore them.

Here's the rules about "slilent" and "loud" syntax:

- Terminals are default to be silent, while non-terminals are loud.
- 

