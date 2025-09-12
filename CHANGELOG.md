# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 新增 `--generate-intermediate-page` 命令行参数，用于控制是否生成中间页
- 新增 `DOVE_GENERATE_INTERMEDIATE_PAGE` 环境变量，用于控制是否生成中间页
- 新增离线功能支持，通过 Service Worker 实现网页离线可用
- 当 `--generate-intermediate-page=false` 时，链接将直接跳转到目标地址，不生成中间页
- 新增 dove.yaml 拆分与 `include` 功能：
  - 主配置支持顶层 `include`/`includes` 字段（字符串或数组），可引用相对路径文件；本地支持通配（glob）。
  - 远程配置（`--features remote`）支持 http(s) include，支持相对 URL，不支持通配。
  - 合并规则：映射（map）递归合并且主文件覆盖；序列（list）按顺序追加；主文件优先级最高。
  - include 文件若为顶层序列，将被视作 `groups: [...]` 片段。
  - 检测循环 include；预览模式会递归监视配置目录，变更片段会触发重建。
- 新增按“分组（group）”设置显示模式：
  - 在 `groups[].display` 指定 `standard|compact|list|text`（支持中文别名）。
  - 仍支持 `site.default_category_display` 作为默认；保留 `site.category_display` 以兼容历史，但优先使用 `groups[].display`。

### Changed
- 优化了命令行参数处理逻辑
- 改进了文档说明

## [0.1.0] - 2025-09-09

### Added
- Initial release
