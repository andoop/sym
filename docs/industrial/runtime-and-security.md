# 6、11、37：运行时、沙箱、合规

## 1. 运行时模型与上限（规划）

- 建议配置项：`max_stack_depth`、`max_locals`、`max_string_bytes`、`max_call_depth`、可选 `wall_time`。
- 当前实现：未统一暴露；VM 见 `local_count`、栈动态增长。

## 2. 沙箱与安全模型（规划）

- 文件 / 网络 / 子进程内建应支持能力列表或策略对象（只读根目录、允许 host 列表等）。
- 默认策略：文档化「当前等同宿主进程权限」。

## 3. 合规与审计日志（规划）

- 对敏感内建（`shell_exec`、`write_file`、`http_post` 等）记录：时间、参数摘要、调用方脚本路径。
- 与组织 SIEM 对接格式（JSON 行）在实现阶段定义。
