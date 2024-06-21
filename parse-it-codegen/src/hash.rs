use hashlink::{LinkedHashMap, LinkedHashSet};
pub use rustc_hash::{FxBuildHasher as Hasher, FxHashMap as HashMap, FxHashSet as HashSet};
pub type OrderedMap<K, V> = LinkedHashMap<K, V, Hasher>;
pub type OrderedSet<K> = LinkedHashSet<K, Hasher>;
