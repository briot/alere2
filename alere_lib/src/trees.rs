use anyhow::Result;

///
/// The data associated with each node
///

pub struct NodeData<K, T> {
    pub key: K,
    pub data: T,

    // The depth of the node in the tree, starting at 0 for root nodes
    pub depth: usize,

    // non-zero if the node was collapsed into its parent (see
    // collapse_if_one_child)
    pub collapse_depth: usize,
}

impl<K, T> NodeData<K, T> {
    pub fn new(key: K, data: T) -> Self {
        NodeData {
            key,
            data,
            depth: 0,
            collapse_depth: 0,
        }
    }
}

///
/// A tree of data
///

pub struct Tree<K, T> {
    roots: NodeList<K, T>,
}

impl<K, T> Default for Tree<K, T> {
    fn default() -> Self {
        Self {
            roots: NodeList::default(),
        }
    }
}

impl<K: PartialEq + Clone, T> Tree<K, T> {
    /// Insert a new node in tree, along with all its parents if they are not
    /// present yet.
    /// Missing parents are created via `create`.
    /// The parents iterator should first return the immediate parent, then
    /// its own parent and so on until the root parent.
    pub fn try_get<F: FnMut(&K) -> T>(
        &mut self,
        key: &K,
        parents: impl Iterator<Item = K>,
        mut create: F,
    ) -> &mut T {
        &mut self.insert_rec(parents, key, &mut create).data.data
    }

    /// Insert a parent node (along with its own parents).  Returns the node.
    /// This is recursive, since we get them via an iterator in the reverse
    /// order.
    fn insert_rec<F: FnMut(&K) -> T>(
        &mut self,
        mut parents: impl Iterator<Item = K>,
        key: &K,
        create: &mut F,
    ) -> &mut TreeNode<K, T> {
        let (depth, parent_list) = match parents.next() {
            None => (0, &mut self.roots),
            Some(p) => {
                let node = self.insert_rec(parents, &p, create);
                (node.data.depth, &mut node.children)
            }
        };
        parent_list.try_get(key, create, depth + 1)
    }
}

impl<K, T> Tree<K, T> {
    /// Sort the tree.
    /// From each row, it extracts one value (as displayed on the screen
    /// presumably), and sort by those values.
    pub fn sort<F, V: Ord>(&mut self, mut get_cell: F)
    where
        F: FnMut(&NodeData<K, T>) -> V,
    {
        self.roots.sort_recursive(&mut |n| get_cell(&n.data));
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
    pub fn traverse_mut<F>(
        &mut self,
        mut process: F,
        parent_first: bool,
    ) -> Result<()>
    where
        F: FnMut(&mut TreeNode<K, T>) -> Result<()>,
    {
        self.roots
            .traverse_recursive_mut(&mut process, parent_first)
    }

    /// Recursively traverse all nodes
    pub fn traverse<F>(&self, mut process: F, parent_first: bool) -> Result<()>
    where
        F: FnMut(&TreeNode<K, T>) -> Result<()>,
    {
        self.roots.traverse_recursive(&mut process, parent_first)
    }
}

///
/// A node in tree.
/// Every node can have children and associated data
///
pub struct TreeNode<K, T> {
    children: NodeList<K, T>,
    pub data: NodeData<K, T>,
}

impl<K, T> TreeNode<K, T> {
    /// Create a new node with no children
    fn new(key: K, data: T, depth: usize) -> Self {
        Self {
            children: NodeList::default(),
            data: NodeData {
                key,
                data,
                depth,
                collapse_depth: 0,
            },
        }
    }

    /// Whether the node has any child
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

impl<K: Clone, T: Clone> TreeNode<K, T> {
    /// Collapse the node: if it has a single child, replace the node by
    /// this child.  This loses any data associated with self though.
    pub fn collapse_if_one_child(&mut self) {
        if self.children.0.len() == 1 {
            let c = &self.children.0[0].data;
            self.data = NodeData {
                key: c.key.clone(),
                data: c.data.clone(),
                depth: self.data.depth,
                collapse_depth: self.data.collapse_depth + c.collapse_depth + 1,
            };
            self.children.0.clear();
        }
    }
}

///
/// A list of nodes
///

struct NodeList<K, T>(Vec<TreeNode<K, T>>);

impl<K, T> Default for NodeList<K, T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K, T> NodeList<K, T> {
    fn sort_recursive<F, V: Ord>(&mut self, get_cell: &mut F)
    where
        F: FnMut(&TreeNode<K, T>) -> V,
    {
        for node in &mut self.0 {
            node.children.sort_recursive(get_cell);
        }
        self.0.sort_by_cached_key(get_cell);
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

    fn traverse_recursive<F>(
        &self,
        process: &mut F,
        parent_first: bool,
    ) -> Result<()>
    where
        F: FnMut(&TreeNode<K, T>) -> Result<()>,
    {
        for node in &self.0 {
            if parent_first {
                process(node)?;
            }
            node.children.traverse_recursive(process, parent_first)?;
            if !parent_first {
                process(node)?;
            }
        }
        Ok(())
    }
    fn traverse_recursive_mut<F>(
        &mut self,
        process: &mut F,
        parent_first: bool,
    ) -> Result<()>
    where
        F: FnMut(&mut TreeNode<K, T>) -> Result<()>,
    {
        for node in self.0.iter_mut() {
            if parent_first {
                process(node)?;
            }
            node.children
                .traverse_recursive_mut(process, parent_first)?;
            if !parent_first {
                process(node)?;
            }
        }
        Ok(())
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
            self.0
                .push(TreeNode::new(key.clone(), create(key), self_depth));
            self.0.last_mut().unwrap()
        }
    }
}
