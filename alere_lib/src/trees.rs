pub struct TreeNode<K, T> {
    children: NodeList<K, T>,
    pub data: NodeData<K, T>,
}

impl<K, T> TreeNode<K, T> {
    fn new(key: K, data: T, depth: usize) -> Self {
        Self {
            children: NodeList::new(),
            data: NodeData { key, data, depth },
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.0.is_empty()
    }

    /// Folds all direct children into an accumulator by applying an operation,
    /// and return the final result.
    pub fn fold<B, F>(&self, init: B, accumulate: F) -> B
    where
        F: FnMut(B, &TreeNode<K, T>) -> B,
    {
        self.children.0.iter().fold(init, accumulate)
    }

    /// Iterate over direct children
    pub fn iter_children(&self) -> impl Iterator<Item = &TreeNode<K, T>> {
        self.children.0.iter()
    }
}

pub struct Tree<K, T> {
    roots: NodeList<K, T>,
}

impl<K, T> Default for Tree<K, T> {
    fn default() -> Self {
        Self {
            roots: NodeList::new(),
        }
    }
}

impl<K: PartialEq + Clone, T> Tree<K, T> {
    pub fn try_get<F>(
        &mut self,
        key: &K,
        parents: &[K], // immediate parent is first in list
        mut create: F,
    ) -> &mut T
    where
        F: FnMut(&K) -> T,
    {
        let mut current = &mut self.roots;
        let mut depth = 0_usize;
        for p in parents.iter().rev() {
            current = &mut current.try_get(p, &mut create, depth).children;
            depth += 1;
        }
        &mut current.try_get(key, &mut create, depth).data.data
    }
}

impl<K, T> Tree<K, T> {
    pub fn sort<F>(&mut self, mut cmp: F)
    where
        F: FnMut(&NodeData<K, T>, &NodeData<K, T>) -> std::cmp::Ordering,
    {
        self.roots
            .sort_recursive(&mut |n1, n2| cmp(&n1.data, &n2.data));
    }

    /// First remove unwanted children, then look at the node itself, so that
    /// the filter can find out whether there remains any children
    pub fn retain<F>(&mut self, mut filter: F)
    where
        F: FnMut(&TreeNode<K, T>) -> bool,
    {
        self.roots.retain_recursive(&mut filter);
    }

    /// Recursively traverse all nodes.
    ///
    /// If parent_first is true, then process is first call on the parent node,
    /// then on all the children.  Otherwise the order is reversed.
    pub fn traverse_mut<F>(&mut self, mut process: F, parent_first: bool)
    where
        F: FnMut(&mut TreeNode<K, T>),
    {
        self.roots
            .traverse_recursive_mut(&mut process, parent_first);
    }

    /// Recursively traverse all nodes
    pub fn traverse<F>(&self, mut process: F, parent_first: bool)
    where
        F: FnMut(&TreeNode<K, T>),
    {
        self.roots.traverse_recursive(&mut process, parent_first);
    }
}

struct NodeList<K, T>(Vec<TreeNode<K, T>>);

impl<K, T> NodeList<K, T> {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn sort_recursive<F>(&mut self, cmp: &mut F)
    where
        F: FnMut(&TreeNode<K, T>, &TreeNode<K, T>) -> std::cmp::Ordering,
    {
        self.0.sort_by(|n1, n2| cmp(n1, n2));
        for node in &mut self.0 {
            node.children.sort_recursive(cmp);
        }
    }

    fn retain_recursive<F>(&mut self, filter: &mut F)
    where
        F: FnMut(&TreeNode<K, T>) -> bool,
    {
        for node in &mut self.0 {
            node.children.retain_recursive(filter);
        }
        self.0.retain(|node| filter(node));
    }

    fn traverse_recursive<F>(&self, process: &mut F, parent_first: bool)
    where
        F: FnMut(&TreeNode<K, T>),
    {
        for node in &self.0 {
            if parent_first {
                process(node);
            }
            node.children.traverse_recursive(process, parent_first);
            if !parent_first {
                process(node);
            }
        }
    }
    fn traverse_recursive_mut<F>(&mut self, process: &mut F, parent_first: bool)
    where
        F: FnMut(&mut TreeNode<K, T>),
    {
        for node in self.0.iter_mut() {
            if parent_first {
                process(node);
            }
            node.children.traverse_recursive_mut(process, parent_first);
            if !parent_first {
                process(node);
            }
        }
    }
}

impl<K: PartialEq + Clone, T> NodeList<K, T> {
    fn try_get<F>(
        &mut self,
        key: &K,
        create: &mut F,
        self_depth: usize,
    ) -> &mut TreeNode<K, T>
    where
        F: FnMut(&K) -> T,
    {
        // Go through an index to avoid issues with the borrow checker
        if let Some(i) = self.0.iter().position(|n| n.data.key == *key) {
            &mut self.0[i]
        } else {
            self.0.push(TreeNode::new(
                key.clone(),
                create(key),
                self_depth + 1,
            ));
            self.0.last_mut().unwrap()
        }
    }
}

pub struct NodeData<K, T> {
    pub key: K,
    pub data: T,

    // The depth of the node in the tree, starting at 0 for root nodes
    pub depth: usize,
}
