use std::collections::HashMap;
use crate::{
    utils::log::Log,
    scene::{
        node::Node,
        base::AsBase,
    },
    core::{
        pool::{
            Handle,
            Pool,
            PoolIterator,
            PoolIteratorMut,
            PoolPairIterator,
            PoolPairIteratorMut,
        },
        math::{
            mat4::Mat4,
            vec3::Vec3,
            vec2::Vec2
        },
        visitor::{
            Visit,
            Visitor,
            VisitResult
        },
    }
};

pub struct Graph {
    root: Handle<Node>,
    pool: Pool<Node>,
    stack: Vec<Handle<Node>>,
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            root: Handle::NONE,
            pool: Pool::new(),
            stack: Vec::new(),
        }
    }
}

impl Graph {
    /// Creates new graph instance with single root node.
    pub fn new() -> Self {
        let mut pool: Pool<Node> = Pool::new();
        let mut root = Node::Base(Default::default());
        root.base_mut().set_name("__ROOT__");
        let root = pool.spawn(root);
        Self {
            stack: Vec::new(),
            root,
            pool,
        }
    }

    /// Adds new node to the graph. Node will be transferred into implementation-defined
    /// storage and you'll get a handle to the node. Node will be automatically attached
    /// to root node of graph, it is required because graph can contain only one root.
    #[inline]
    pub fn add_node(&mut self, node: Node) -> Handle<Node> {
        let handle = self.pool.spawn(node);
        self.link_nodes(handle, self.root);
        handle
    }

    /// Tries to borrow shared reference to a node by specified handle. Will panic if handle
    /// is invalid. Handle can be invalid for either because its index out-of-bounds or generation
    /// of handle does not match generation of node.
    pub fn get(&self, node: Handle<Node>) -> &Node {
        self.pool.borrow(node)
    }

    /// Tries to borrow mutable reference to a node by specified handle. Will panic if handle
    /// is invalid. Handle can be invalid for either because its index out-of-bounds or generation
    /// of handle does not match generation of node.
    pub fn get_mut(&mut self, node: Handle<Node>) -> &mut Node {
        self.pool.borrow_mut(node)
    }

    /// Tries to borrow mutable references to two nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    pub fn get_two_mut(&mut self, nodes: (Handle<Node>, Handle<Node>))
                       -> (&mut Node, &mut Node) {
        self.pool.borrow_two_mut(nodes)
    }

    /// Tries to borrow mutable references to three nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    pub fn get_three_mut(&mut self, nodes: (Handle<Node>, Handle<Node>, Handle<Node>))
                         -> (&mut Node, &mut Node, &mut Node) {
        self.pool.borrow_three_mut(nodes)
    }

    /// Tries to borrow mutable references to four nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    pub fn get_four_mut(&mut self, nodes: (Handle<Node>, Handle<Node>, Handle<Node>, Handle<Node>))
                        -> (&mut Node, &mut Node, &mut Node, &mut Node) {
        self.pool.borrow_four_mut(nodes)
    }

    /// Returns root node of current graph.
    pub fn get_root(&self) -> Handle<Node> {
        self.root
    }

    /// Destroys node and its children recursively.
    #[inline]
    pub fn remove_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);

        self.stack.clear();
        self.stack.push(node_handle);
        while let Some(handle) = self.stack.pop() {
            let base = self.pool.borrow(handle).base();
            for child in base.children().iter() {
                self.stack.push(*child);
            }
            self.pool.free(handle);
        }
    }

    fn unlink_internal(&mut self, node_handle: Handle<Node>) {
        // Replace parent handle of child
        let node = self.pool.borrow_mut(node_handle);
        let parent_handle = node.base().parent;
        node.base_mut().parent = Handle::NONE;

        // Remove child from parent's children list
        if parent_handle.is_some() {
            let parent = self.pool.borrow_mut(parent_handle);
            if let Some(i) = parent.base().children().iter().position(|h| *h == node_handle) {
                parent.base_mut().children.remove(i);
            }
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<Node>, parent_handle: Handle<Node>) {
        self.unlink_internal(child_handle);
        let child = self.pool.borrow_mut(child_handle);
        child.base_mut().parent = parent_handle;
        let parent = self.pool.borrow_mut(parent_handle);
        parent.base_mut().children.push(child_handle);
    }

    /// Unlinks specified node from its parent and attaches it to root graph node.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);
        self.link_nodes(node_handle, self.root);
        self.get_mut(node_handle)
            .base_mut()
            .local_transform_mut()
            .set_position(Vec3::ZERO);
    }

    /// Tries to find a copy of `node_handle` in hierarchy tree starting from `root_handle`.
    pub fn find_copy_of(&self, root_handle: Handle<Node>, node_handle: Handle<Node>) -> Handle<Node> {
        let root = self.pool.borrow(root_handle);
        if root.base().original_handle() == node_handle {
            return root_handle;
        }

        for child_handle in root.base().children() {
            let out = self.find_copy_of(*child_handle, node_handle);
            if out.is_some() {
                return out;
            }
        }

        Handle::NONE
    }

    /// Searches node with specified name starting from specified node. If nothing was found,
    /// [`Handle::NONE`] is returned.
    pub fn find_by_name(&self, root_node: Handle<Node>, name: &str) -> Handle<Node> {
        let base = self.pool.borrow(root_node).base();
        if base.name() == name {
            root_node
        } else {
            let mut result: Handle<Node> = Handle::NONE;
            for child in base.children() {
                let child_handle = self.find_by_name(*child, name);
                if !child_handle.is_none() {
                    result = child_handle;
                    break;
                }
            }
            result
        }
    }

    /// Searches node with specified name starting from root. If nothing was found, `Handle::NONE`
    /// is returned.
    pub fn find_by_name_from_root(&self, name: &str) -> Handle<Node> {
        self.find_by_name(self.root, name)
    }

    /// Creates deep copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns `Handle::NONE`. This method can be used
    /// to create exact copy of given node hierarchy. For example you can prepare rocket
    /// model: case of rocket will be mesh, and fire from nozzle will be particle system,
    /// and when you fire from rocket launcher you just need to create a copy of such
    /// "prefab".
    ///
    /// # Notes
    ///
    /// This method does *not* copy any animations! You have to copy them manually. In most
    /// cases it is fine to retarget animation from a resource you want, it will create
    /// animation copy from resource that will work with your nodes hierarchy.
    ///
    /// # Implementation notes
    ///
    /// This method automatically remaps bones for copied surfaces.
    pub fn copy_node(&self, node_handle: Handle<Node>, dest_graph: &mut Graph) -> Handle<Node> {
        let mut old_new_mapping: HashMap<Handle<Node>, Handle<Node>> = HashMap::new();
        let root_handle = self.copy_node_raw(node_handle, dest_graph, &mut old_new_mapping);

        // Iterate over instantiated nodes and remap bones handles.
        for (_, new_node_handle) in old_new_mapping.iter() {
            if let Node::Mesh(mesh) = dest_graph.pool.borrow_mut(*new_node_handle) {
                for surface in mesh.surfaces_mut() {
                    for bone_handle in surface.bones.iter_mut() {
                        if let Some(entry) = old_new_mapping.get(bone_handle) {
                            *bone_handle = *entry;
                        }
                    }
                }
            }
        }

        root_handle
    }

    fn copy_node_raw(&self, root_handle: Handle<Node>, dest_graph: &mut Graph, old_new_mapping: &mut HashMap<Handle<Node>, Handle<Node>>) -> Handle<Node> {
        let src_node = self.pool.borrow(root_handle);
        let mut dest_node = src_node.clone();
        dest_node.base_mut().original = root_handle;
        let dest_copy_handle = dest_graph.add_node(dest_node);
        old_new_mapping.insert(root_handle, dest_copy_handle);
        for src_child_handle in src_node.base().children() {
            let dest_child_handle = self.copy_node_raw(*src_child_handle, dest_graph, old_new_mapping);
            if !dest_child_handle.is_none() {
                dest_graph.link_nodes(dest_child_handle, dest_copy_handle);
            }
        }
        dest_copy_handle
    }

    /// Searches root node in given hierarchy starting from given node. This method is used
    /// when you need to find a root node of a model in complex graph.
    fn find_model_root(&self, from: Handle<Node>) -> Handle<Node> {
        let mut model_root_handle = from;
        while model_root_handle.is_some() {
            let model_node = self.pool.borrow(model_root_handle).base();

            if model_node.parent().is_none() {
                // We have no parent on node, then it must be root.
                return model_root_handle;
            }

            if model_node.is_resource_instance() {
                return model_root_handle;
            }

            // Continue searching up on hierarchy.
            model_root_handle = model_node.parent();
        }
        model_root_handle
    }

    pub(in crate) fn resolve(&mut self) {
        Log::writeln("Resolving graph...".to_owned());

        self.update_transforms();

        // Resolve original handles. Original handle is a handle to a node in resource from which
        // a node was instantiated from. We can resolve it only by names of nodes, but this is not
        // reliable way of doing this, because some editors allow nodes to have same names for
        // objects, but here we'll assume that modellers will not create models with duplicated
        // names.
        for node in self.pool.iter_mut() {
            let base = node.base_mut();
            if let Some(model) = base.resource() {
                let model = model.lock().unwrap();
                for (handle, resource_node) in model.get_scene().graph.pair_iter() {
                    if resource_node.base().name() == base.name() {
                        base.original = handle;
                        base.inv_bind_pose_transform = resource_node.base().inv_bind_pose_transform();
                        break;
                    }
                }
            }
        }

        Log::writeln("Original handles resolved!".to_owned());

        // Taking second reference to self is safe here because we need it only
        // to iterate over graph and find copy of bone node. We won't modify pool
        // while iterating over it, so it is double safe.
        let graph = unsafe { &*(self as *const Graph) };

        // Then iterate over all scenes and resolve changes in surface data, remap bones, etc.
        // This step is needed to take correct graphical data from resource, we do not store
        // meshes in save files, just references to resource this data was taken from. So on
        // resolve stage we just copying surface from resource, do bones remapping. Bones remapping
        // is required stage because we copied surface from resource and bones are mapped to nodes
        // in resource, but we must have them mapped to instantiated nodes on scene. To do that
        // we'll try to find a root for each node, and starting from it we'll find corresponding
        // bone nodes. I know that this sounds too confusing but try to understand it.
        for (node_handle, node) in self.pool.pair_iter_mut() {
            if let Node::Mesh(mesh) = node {
                let root_handle = graph.find_model_root(node_handle);
                let node_name = String::from(mesh.base().name());
                if let Some(model) = mesh.base().resource() {
                    let model = model.lock().unwrap();
                    let resource_node_handle = model.find_node_by_name(node_name.as_str());
                    if let Node::Mesh(resource_mesh) = model.get_scene().graph.get(resource_node_handle) {
                        // Copy surfaces from resource and assign to meshes.
                        mesh.clear_surfaces();
                        for resource_surface in resource_mesh.surfaces() {
                            mesh.add_surface(resource_surface.clone());
                        }

                        // Remap bones
                        for surface in mesh.surfaces_mut() {
                            for bone_handle in surface.bones.iter_mut() {
                                *bone_handle = graph.find_copy_of(root_handle, *bone_handle);
                            }
                        }
                    }
                }
            }
        }

        Log::writeln("Graph resolved successfully!".to_owned());
    }

    pub fn update_transforms(&mut self) {
        // Calculate transforms on nodes
        self.stack.clear();
        self.stack.push(self.root);
        while let Some(handle) = self.stack.pop() {
            // Calculate local transform and get parent handle
            let parent_handle = self.pool.borrow_mut(handle).base().parent();

            let (parent_global_transform, parent_visibility) =
                if parent_handle.is_some() {
                    let parent = self.pool.borrow(parent_handle).base();
                    (parent.global_transform(), parent.global_visibility())
                } else {
                    (Mat4::IDENTITY, true)
                };

            let base = self.pool.borrow_mut(handle).base_mut();
            base.global_transform = parent_global_transform * base.local_transform().matrix();
            base.global_visibility = parent_visibility && base.visibility();

            // Queue children and continue traversal on them
            for child_handle in base.children() {
                self.stack.push(child_handle.clone());
            }
        }
    }

    pub fn is_valid_handle(&self, node_handle: Handle<Node>) -> bool {
        self.pool.is_valid_handle(node_handle)
    }

    pub fn update_nodes(&mut self, frame_size: Vec2, dt: f32) {
        self.update_transforms();

        for node in self.pool.iter_mut() {
            if let Some(lifetime) = node.base().lifetime() {
                node.base_mut().set_lifetime(lifetime - dt);
            }

            match node {
                Node::Camera(camera) => camera.calculate_matrices(frame_size),
                Node::ParticleSystem(particle_system) => particle_system.update(dt),
                _ => ()
            }
        }

        for i in 0..self.pool.get_capacity() {
            let remove = if let Some(node) = self.pool.at(i) {
                if let Some(lifetime) = node.base().lifetime() {
                    lifetime <= 0.0
                } else {
                    false
                }
            } else {
                continue;
            };

            if remove {
                self.remove_node(self.pool.handle_from_index(i));
            }
        }
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    pub fn linear_iter(&self) -> PoolIterator<Node> {
        self.pool.iter()
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    pub fn linear_iter_mut(&mut self) -> PoolIteratorMut<Node> {
        self.pool.iter_mut()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    pub fn pair_iter(&self) -> PoolPairIterator<Node> {
        self.pool.pair_iter()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    pub fn pair_iter_mut(&mut self) -> PoolPairIteratorMut<Node> {
        self.pool.pair_iter_mut()
    }

    /// Create graph depth traversal iterator.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    pub fn traverse_iter(&self, from: Handle<Node>) -> GraphTraverseIterator {
        GraphTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Create graph depth traversal iterator which will emit *handles* to nodes.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    pub fn traverse_handle_iter(&self, from: Handle<Node>) -> GraphHandleTraverseIterator {
        GraphHandleTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }
}

pub struct GraphTraverseIterator<'a> {
    graph: &'a Graph,
    stack: Vec<Handle<Node>>,
}

impl<'a> Iterator for GraphTraverseIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            let node = self.graph.get(handle);

            for child_handle in node.base().children() {
                self.stack.push(*child_handle);
            }

            return Some(node);
        }

        None
    }
}

pub struct GraphHandleTraverseIterator<'a> {
    graph: &'a Graph,
    stack: Vec<Handle<Node>>,
}

impl<'a> Iterator for GraphHandleTraverseIterator<'a> {
    type Item = Handle<Node>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            for child_handle in self.graph.get(handle).base().children() {
                self.stack.push(*child_handle);
            }

            return Some(handle);
        }
        None
    }
}

impl Visit for Graph {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Pool must be empty, otherwise handles will be invalid and everything will blow up.
        if visitor.is_reading() && self.pool.get_capacity() != 0 {
            panic!("Graph pool must be empty on load!")
        }

        self.root.visit("Root", visitor)?;
        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        scene::{
            graph::Graph,
            node::Node,
            base::Base,
        },
        core::pool::Handle
    };

    #[test]
    fn graph_init_test() {
        let graph = Graph::new();
        assert_ne!(graph.root, Handle::NONE);
        assert_eq!(graph.pool.alive_count(), 1);
    }

    #[test]
    fn graph_node_test() {
        let mut graph = Graph::new();
        let a = graph.add_node(Node::Base(Base::default()));
        let b = graph.add_node(Node::Base(Base::default()));
        let c = graph.add_node(Node::Base(Base::default()));
        assert_eq!(graph.pool.alive_count(), 4);
    }
}