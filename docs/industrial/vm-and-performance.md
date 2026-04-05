# 5、19、21、22：VM 路线、基准、后端、缓存

## 1. VM 特性扩展（相对 37 步第 5 项）

- 优先级建议：`match` 受限形式 → ADT 判别 → 更多内建下沉为指令。

## 2. 性能基准（规划）

- 用例：`fib`、`字符串 concat 循环`、`println` 吞吐；对比 `--vm` 与树解释器。
- 脚本：`scripts/smoke_bench.sh`（可选）。

## 3. VM 后端演进

- 阶段 A：指令线程化 / 跳转表。
- 阶段 B：寄存器 IR。
- 阶段 C：可选 JIT（cranelift/llvm），仅对稳定子集。

## 4. 增量编译 / 缓存

- 按 `(文件内容哈希, 编译器版本)` 缓存 `Program` 字节码；CLI `sym build-cache`。
