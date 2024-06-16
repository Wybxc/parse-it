use std::{fmt::Display, ops::Index};

use hashlink::{linked_hash_map::CursorMut, LinkedHashMap};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;

use crate::Hasher;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Value(pub u32);

pub enum Capture {
    Loud,
    Slient,
    Named(Box<syn::Pat>, Box<Capture>),
    Tuple(Box<Capture>, Box<Capture>),
}

impl Capture {
    pub fn is_loud(&self) -> bool {
        match self {
            Capture::Loud => true,
            Capture::Slient => false,
            Capture::Named(_, _) => true,
            Capture::Tuple(_, n) => n.is_loud(),
        }
    }

    pub fn unify(self, cap: Capture) -> Result<Capture, TokenStream> {
        match (self, cap) {
            (Capture::Named(p1, c1), Capture::Named(p2, c2)) => {
                if p1 == p2 {
                    if let Ok(c) = c1.unify(*c2) {
                        Ok(Capture::Named(p1, Box::new(c)))
                    } else {
                        Ok(Capture::Named(p1, Box::new(Capture::Loud)))
                    }
                } else {
                    Err(quote_spanned! {
                        p1.span() => compile_error!("pattern mismatch")
                    })
                }
            }
            (Capture::Tuple(c1, c2), Capture::Tuple(c3, c4)) => {
                let c1 = c1.unify(*c3)?;
                let c2 = c2.unify(*c4)?;
                Ok(Capture::Tuple(Box::new(c1), Box::new(c2)))
            }
            (Capture::Loud, _) => Ok(Capture::Loud),
            (_, Capture::Loud) => Ok(Capture::Loud),
            (Capture::Slient, Capture::Slient) => Ok(Capture::Slient),
            _ => Err(quote! {
                compile_error!("capture mismatch")
            }),
        }
    }
}

pub struct ValueData {
    kind: ValueKind,
}

impl ValueData {
    pub fn kind(&self) -> &ValueKind {
        &self.kind
    }

    pub fn declare() -> Self {
        Self {
            kind: ValueKind::Declare,
        }
    }

    pub fn define(decl: Value, value: Value) -> Self {
        Self {
            kind: ValueKind::Define { decl, value },
        }
    }

    pub fn just(c: char) -> (Self, Capture) {
        let just = Self {
            kind: ValueKind::Just(c),
        };
        (just, Capture::Slient)
    }

    pub fn map(v: Value, c: Capture, t: syn::Type, e: syn::Expr) -> Self {
        Self {
            kind: ValueKind::Map(v, c, t, e),
        }
    }

    pub fn then((v1, cap1): (Value, Capture), (v2, cap2): (Value, Capture)) -> (Self, Capture) {
        let loud1 = cap1.is_loud();
        let loud2 = cap2.is_loud();
        if loud1 && !loud2 {
            let kind = ValueKind::ThenIgnore(v1, v2);
            return (Self { kind }, cap1);
        }
        if !loud1 && loud2 {
            let kind = ValueKind::IgnoreThen(v1, v2);
            return (Self { kind }, cap2);
        }

        let then = Self {
            kind: ValueKind::Then(v1, v2),
        };
        (then, Capture::Tuple(Box::new(cap1), Box::new(cap2)))
    }

    /// # Panics
    /// Panics if the input vector is empty.
    pub fn choice(
        mut vs: impl Iterator<Item = Result<(Value, Capture), TokenStream>>,
    ) -> Result<(Self, Capture), TokenStream> {
        let (v, cap) = vs.next().unwrap()?;
        let mut acc = vec![v];
        let mut u = cap;

        for v in vs {
            let (v, c) = v?;
            u = c.unify(u)?;
            acc.push(v);
        }

        let kind = ValueKind::Choice(acc);
        Ok((Self { kind }, u))
    }

    pub fn choice_nocap(vs: Vec<Value>) -> Self {
        Self {
            kind: ValueKind::Choice(vs),
        }
    }

    pub fn repeat((v, cap): (Value, Capture)) -> (Self, Capture) {
        let kind = ValueKind::Repeat(v);
        let cap = if cap.is_loud() {
            Capture::Loud
        } else {
            Capture::Slient
        };
        (Self { kind }, cap)
    }

    pub fn repeat1((v, cap): (Value, Capture)) -> (Self, Capture) {
        let kind = ValueKind::Repeat1(v);
        let cap = if cap.is_loud() {
            Capture::Loud
        } else {
            Capture::Slient
        };
        (Self { kind }, cap)
    }

    pub fn or_not((v, cap): (Value, Capture)) -> (Self, Capture) {
        let kind = ValueKind::OrNot(v);
        let cap = if cap.is_loud() {
            Capture::Loud
        } else {
            Capture::Slient
        };
        (Self { kind }, cap)
    }
}

/// IR should be type-ignorant
///
/// ```text
/// Declare : ref Parser (+)
/// Define : ref Parser _ -> Parser _ -> ()
/// Just : char -> Parser (-)
/// Noise : exists n, Parser _ -> Parser n
/// Map : Parser n -> Expr -> Parser (+)
/// Then : Parser m -> Parser n -> Parser (m, n)
/// ThenIgnore : Parser m -> Parser _ -> Parser m
/// IgnoreThen : Parser _ -> Parser n -> Parser n
/// Choice : [Parser n] -> Parser n
/// Repeat : Parser n -> Parser (+)
/// Repeat1 : Parser n -> Parser (+)
/// OrNot : Parser n -> Parser (+)
///
///
/// ```
pub enum ValueKind {
    Declare,
    Define { decl: Value, value: Value },
    Just(char),
    Map(Value, Capture, syn::Type, syn::Expr),

    Then(Value, Value),
    ThenIgnore(Value, Value),
    IgnoreThen(Value, Value),
    Choice(Vec<Value>),
    Repeat(Value),
    Repeat1(Value),
    OrNot(Value),
}

impl ValueKind {
    pub fn uses(&self) -> Vec<Value> {
        match self {
            ValueKind::Declare => vec![],
            ValueKind::Define { decl, value } => vec![*decl, *value],
            ValueKind::Just(_) => vec![],
            ValueKind::Map(v, _, _, _) => vec![*v],
            ValueKind::Then(v1, v2) => vec![*v1, *v2],
            ValueKind::ThenIgnore(v1, v2) => vec![*v1, *v2],
            ValueKind::IgnoreThen(v1, v2) => vec![*v1, *v2],
            ValueKind::Choice(vs) => vs.clone(),
            ValueKind::Repeat(v) => vec![*v],
            ValueKind::Repeat1(v) => vec![*v],
            ValueKind::OrNot(v) => vec![*v],
        }
    }
}

/// Middle representation of the parser.
#[derive(Default)]
pub struct Middle {
    next_value: u32,
    values: LinkedHashMap<Value, ValueData, Hasher>,
    pub results: Vec<Value>,
}

impl Middle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_back(&mut self, data: ValueData) -> Value {
        let value = Value(self.next_value);
        self.next_value += 1;

        self.values.insert(value, data);
        value
    }

    pub fn value(&self, value: Value) -> Option<&ValueData> {
        self.values.get(&value)
    }

    pub fn values(&self) -> impl Iterator<Item = (&Value, &ValueData)> {
        self.values.iter()
    }

    pub fn cursor_front_mut(&mut self) -> CursorMut<Value, ValueData, Hasher> {
        self.values.cursor_front_mut()
    }

    pub fn cursor_back_mut(&mut self) -> CursorMut<Value, ValueData, Hasher> {
        self.values.cursor_back_mut()
    }

    pub fn debug(self) -> MiddleDebug {
        MiddleDebug(self)
    }

    pub fn debug_with(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (value, data) in self.values.iter() {
            match &data.kind {
                ValueKind::Declare => writeln!(fmt, "#{} = Declare", value.0)?,
                ValueKind::Define { decl, value } => {
                    writeln!(fmt, "Define #{} = #{}", decl.0, value.0)?
                }
                ValueKind::Just(c) => writeln!(fmt, "#{} = Just '{}'", value.0, c)?,
                ValueKind::Map(v, _, _, e) => {
                    writeln!(fmt, "#{} = Map #{} {}", value.0, v.0, e.to_token_stream())?
                }
                ValueKind::Then(v1, v2) => {
                    writeln!(fmt, "#{} = Then #{} #{}", value.0, v1.0, v2.0)?
                }
                ValueKind::ThenIgnore(v1, v2) => {
                    writeln!(fmt, "#{} = ThenIgnore #{}, #{}", value.0, v1.0, v2.0)?
                }
                ValueKind::IgnoreThen(v1, v2) => {
                    writeln!(fmt, "#{} = IgnoreThen #{}, #{}", value.0, v1.0, v2.0)?
                }
                ValueKind::Choice(vs) => {
                    write!(fmt, "#{} = Choice [", value.0)?;
                    for v in vs.iter() {
                        write!(fmt, "#{} ", v.0)?;
                    }
                    writeln!(fmt, "]")?
                }
                ValueKind::Repeat(v) => writeln!(fmt, "#{} = Repeat #{}", value.0, v.0)?,
                ValueKind::Repeat1(v) => writeln!(fmt, "#{} = Repeat1 #{}", value.0, v.0)?,
                ValueKind::OrNot(v) => writeln!(fmt, "#{} = OrNot #{}", value.0, v.0)?,
            }
        }
        if self.results.len() == 1 {
            writeln!(fmt, "Return #{}", self.results[0].0)?
        } else {
            write!(fmt, "Return (")?;
            for (i, v) in self.results.iter().enumerate() {
                write!(fmt, "#{}", v.0)?;
                if i != self.results.len() - 1 {
                    write!(fmt, ", ")?;
                }
            }
            writeln!(fmt, ")")?
        }
        Ok(())
    }
}

impl Index<Value> for Middle {
    type Output = ValueData;

    fn index(&self, index: Value) -> &Self::Output {
        self.values.get(&index).unwrap()
    }
}

pub struct MiddleDebug(pub Middle);

impl Display for MiddleDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.debug_with(f)
    }
}
