use std::collections::HashMap;

#[derive(Default)]
pub struct TrieNode {
    pub children: HashMap<char, TrieNode>,
    pub indices: Vec<usize>,
}

#[derive(Default)]
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a string into the trie, associating it with the given entry index.
    pub fn insert(&mut self, text: &str, index: usize) {
        let text = text.to_lowercase();
        // Insert the full string and also every token for better matching
        self.insert_sequence(&text, index);
        for token in text.split_whitespace() {
            self.insert_sequence(token, index);
        }
    }

    fn insert_sequence(&mut self, seq: &str, index: usize) {
        let mut node = &mut self.root;
        for c in seq.chars() {
            node = node.children.entry(c).or_default();
            if node.indices.last() != Some(&index) {
                node.indices.push(index);
            }
        }
    }

    /// Returns a list of indices matching the prefix.
    pub fn search(&self, prefix: &str) -> Vec<usize> {
        let mut node = &self.root;
        let prefix = prefix.to_lowercase();
        for c in prefix.chars() {
            match node.children.get(&c) {
                Some(child) => node = child,
                None => return vec![],
            }
        }
        node.indices.clone()
    }
}
