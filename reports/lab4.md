





## Root inode在easy-fs中的作用

在easy-fs中，root inode（根索引节点）起着至关重要的作用：

1. **文件系统入口点**：root inode是整个文件系统的入口点，它代表根目录"/"。在efs.rs的第111-117行，我们可以看到`root_inode`方法，它返回inode_id为0的inode，这是文件系统创建时分配的第一个inode。
```rust
/// Get the root inode of the filesystem
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }   // 文件系统的使用者在通过 EasyFileSystem::open 从装载了 easy-fs 镜像的块设备上打开 easy-fs 之后，要做的第一件事情就是获取根目录的 Inode 。因为我们目前仅支持绝对路径，对于任何文件/目录的索引都必须从根目录开始向下逐级进行。等到索引完成之后，我们才能对文件/目录进行操作。
```

2. **目录结构起点**：所有文件和目录的访问都必须从root inode开始。在vfs.rs的第85行注释中明确指出："find方法只会被根目录Inode调用，文件系统中其他文件的Inode不会调用这个方法"。这表明所有文件查找操作都从root inode开始。
```rust
/// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }       // find 方法只会被根目录 Inode 调用，文件系统中其他文件的 Inode 不会调用这个方法。它首先调用 find_inode_id 方法尝试从根目录的 DiskInode 上找到要索引的文件名对应的 inode 编号。这就需要将根目录内容中的所有目录项都读到内存进行逐个比对。如果能够找到的话， find 方法会根据查到 inode 编号对应生成一个 Inode 用于后续对文件的访问。
```

3. **文件系统初始化的关键部分**：在efs.rs的第77-84行，创建文件系统时会初始化root inode，将其设置为目录类型。这是文件系统创建过程中的关键步骤。
```rust
// 我们要做的事情是创建根目录 / 。首先需要调用 alloc_inode 在 inode 位图中分配一个 inode ，由于这是第一次分配，它的编号固定是 0 。接下来需要将分配到的 inode 初始化为 easy-fs 中的唯一一个目录，我们需要调用 get_disk_inode_pos 来根据 inode 编号获取该 inode 所在的块的编号以及块内偏移，之后就可以将它们传给 get_block_cache 和 modify 了。
        // write back immediately
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
```

4. **路径解析的基础**：由于easy-fs只支持绝对路径，任何文件或目录的访问都必须从root inode开始逐级向下查找。

## 如果root inode内容损坏会发生什么

如果root inode的内容损坏，将会导致严重的后果：

1. **文件系统无法访问**：由于所有文件和目录的访问都必须从root inode开始，如果root inode损坏，整个文件系统将变得不可访问。系统无法定位任何文件或目录，包括系统关键文件。

2. **目录结构丢失**：root inode包含根目录的内容，即指向其他文件和目录的目录项。如果这些内容损坏，整个文件系统的目录结构将丢失，即使文件数据本身可能仍然存在于磁盘上。

3. **文件系统挂载失败**：在efs.rs的第89-109行的`open`方法中，当尝试打开文件系统时，如果root inode损坏，可能会导致文件系统挂载失败。系统可能无法识别这是一个有效的文件系统。
```rust
/// Open a block device as a filesystem
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {   // 通过 open 方法可以从一个已写入了 easy-fs 镜像的块设备上打开我们的 easy-fs
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }   // 它只需将块设备编号为 0 的块作为超级块读取进来，就可以从中知道 easy-fs 的磁盘布局，由此可以构造 efs 实例。
```

4. **数据恢复困难**：由于root inode损坏，即使文件数据仍然存在于磁盘上，也很难恢复这些数据，因为失去了文件与数据块之间的关联信息。

5. **系统崩溃**：如果操作系统依赖这个文件系统存储关键文件，root inode损坏可能导致系统无法启动或运行时崩溃。

在efs.rs的第94行有一个检查`assert!(super_block.is_valid(), "Error loading EFS!");`，但这只是检查超级块的有效性，并没有专门检查root inode的完整性。因此root inode损坏可能不会被立即检测到，而是在试图访问文件系统时才会发现问题。
