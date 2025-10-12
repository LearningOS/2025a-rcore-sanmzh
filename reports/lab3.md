




## stride 算法深入

STRIDE 算法是一种调度算法，其核心思想是：
- 每个进程维护一个 stride 值
- 每次选择 stride 值最小的进程运行
- 进程运行一个时间片后，其 stride 值增加 BigStride/优先级

---

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

### 实际情况是轮到 p1 执行吗？为什么？

在给定的例子中：
- p1.stride = 255
- p2.stride = 250
使用 8bit 无符号整数存储
当 p2 执行一个时间片后，其 stride 值会增加 BigStride/优先级。假设优先级为 10，则增加量为 255/10 ≈ 25。
所以 p2.stride = 250 + 25 = 275，但由于是 8bit 存储，实际值为 275 mod 256 = 19。
此时： 
- p1.stride = 255
- p2.stride = 19
按照正常比较，19 < 255，所以应该选择 p2，但这与理论预期（应该选择 p1）不符。

---

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

### 为什么？尝试简单说明（不要求严格证明）。

当所有进程优先级 >= 2 时，可以证明 STRIDE_MAX - STRIDE_MIN <= BigStride/2。
简单解释：
- 每次增加量至少为 BigStride/2（因为优先级 <= 2）
- 两个进程的 stride 值差距不会超过 BigStride/2
- 这样即使发生溢出，也能正确比较大小

```markdown
让我更详细地解释为什么在进程优先级全部 >= 2 的情况下，STRIDE_MAX - STRIDE_MIN <= BigStride/2。

## STRIDE 算法中的优先级与步长关系

首先，我们需要明确 STRIDE 算法中优先级和步长的关系：

1. **步长计算公式**：
   ```
   stride_increment = BigStride / priority
   ```
   其中 BigStride 是一个固定值（如 255 或 2^64-1），priority 是进程的优先级。

2. **优先级与步长的反比关系**：
   - 优先级越高，步长越小（进程调度更频繁）
   - 优先级越低，步长越大（进程调度较少）

## 为什么优先级 >= 2 能限制 STRIDE 差值

### 1. 最大步长的限制

当所有进程优先级 >= 2 时：
- 最大步长 = BigStride / 2（当优先级 = 2 时）
- 这意味着任何进程每次调度后，其 stride 值最多增加 BigStride/2

### 2. STRIDE 差值的限制

考虑两个进程 A 和 B，假设：
- A 是当前 STRIDE_MAX（最大 stride 值的进程）
- B 是当前 STRIDE_MIN（最小 stride 值的进程）

当 B 运行一个时间片后：
- B.stride = B.stride + BigStride/priority_B
- 由于 priority_B >= 2，所以 B.stride 最多增加 BigStride/2

此时，可能的情况：
1. 如果 B.stride + BigStride/priority_B 仍然 <= A.stride，那么 B 仍然是 STRIDE_MIN
2. 如果 B.stride + BigStride/priority_B > A.stride，那么 B 成为新的 STRIDE_MAX

无论哪种情况，STRIDE_MAX - STRIDE_MIN 都不会超过 BigStride/2。

### 3. 数学解释

假设初始时：
- STRIDE_MAX = S_max
- STRIDE_MIN = S_min

当 STRIDE_MIN 的进程运行后：
- 新的 STRIDE_MIN' = S_min + BigStride/priority_min
- 由于 priority_min >= 2，所以 STRIDE_MIN' <= S_min + BigStride/2

如果 STRIDE_MIN' <= S_max，则：
- 新的 STRIDE_MAX - STRIDE_MIN = S_max - STRIDE_MIN' >= S_max - (S_min + BigStride/2) = (S_max - S_min) - BigStride/2

如果 STRIDE_MIN' > S_max，则：
- 新的 STRIDE_MAX = STRIDE_MIN'
- 新的 STRIDE_MIN = S_max
- 新的 STRIDE_MAX - STRIDE_MIN = STRIDE_MIN' - S_max <= (S_min + BigStride/2) - S_max = BigStride/2 - (S_max - S_min)

无论哪种情况，STRIDE_MAX - STRIDE_MIN 都不会超过 BigStride/2。

### 4. 直观理解

可以想象一个时钟：
- 所有进程的 stride 值都在时钟上
- 每次调度，当前最小 stride 值的进程"前进"一定步长
- 由于步长最大不超过 BigStride/2，所以任何两个进程在时钟上的"距离"不会超过 BigStride/2

## 为什么这个性质很重要

这个性质保证了：
1. 即使发生溢出，我们也能正确比较两个 stride 值的大小
2. 可以使用简单的比较器来确定哪个进程应该被调度
3. 避免了复杂的溢出处理逻辑

这就是为什么 STRIDE 调度算法通常要求进程优先级 >= 2 的原因。
```

---

已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。


对于溢出情况，我们需要一个特殊的比较器。关键思路是：
- 如果两个值的差值小于 BigStride/2，则正常比较
- 如果差值大于 BigStride/2，则说明发生了溢出，需要反向比较
```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 假设 BigStride 为 2^64 - 1
		const BIG_STRIDE: u64 = u64::MAX;
		const HALF_BIG_STRIDE: u64 = BIG_STRIDE / 2;

		let diff = self.0.wrapping_sub(othre.0);

		if diff < HALF_BIG_STRIDE {
			// 正常情况， self > other
			Some(Ordering::Greater)
		} else {
			// 移除情况， self < other
			Some(Ordering::Less)
		}
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}


// 以 8bit 为例，BigStride = 255，HalfBigStride = 127：

// 比较 125 和 255：
// diff = 125 - 255 = -124 mod 256 = 132
// 132 > 127，所以 125 > 255（溢出情况，反向比较）

// 比较 129 和 255：
// diff = 129 - 255 = -126 mod 256 = 130
// 130 > 127，所以 129 > 255（溢出情况，反向比较）
```
TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: (125 < 255) == false, (129 < 255) == true.
