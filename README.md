# dove

一个用 Rust 生成静态网页的简洁导航站点生成器，输出可直接部署到 Cloudflare Pages。支持主题目录与模板引擎（Tera），可自定义 HTML/CSS/JS；支持内/外网两套页面。

## 快速开始

- 生成示例配置与默认主题：

  `cargo run -- init`

- 根据配置生成静态站点到 `dist/`：

 `cargo run -- build`

- 启动本地预览（默认监听 `127.0.0.1:8787`，启动前会先构建一次）：

  `cargo run -- preview --build-first`

生成完成后，打开 `dist/index.html`（外网版）或 `dist/intranet.html`（内网版）预览即可；若配置了 `base_path`，则文件位于 `dist/<base_path>/` 下。页头可相互切换。

如需“本地仅保留模板、配置放在 Gist”：不放置 `dove.yaml`，用 `--input-url` 或环境变量 `DOVE_INPUT_URL`/`DOVE_GIST_ID` 指定远程配置。
注意：远程加载功能位于可选特性 `remote` 中，默认未启用；请使用 `--features remote`。

## 配置说明（dove.yaml）

示例：

```
site:
  title: 我的导航站
  description: 常用站点与工具集合
  color_scheme: auto   # auto | light | dark
  theme_dir: themes/default
  base_path: secretPath   # 可选：将站点生成到 dist/secretPath/
  redirect:               # 跳转页设置（仅外网链接生成中间页）
    delay_seconds: 3      # 自动跳转倒计时（秒），0 表示不自动跳转
    default_risk: low     # 默认风险等级：low|medium|high
    utm:                  # 站点级 UTM（可选；字段留空则不追加）
      source: nav
      medium: referral
      campaign: homepage

groups:
  - name: 常用
    links:
      - name: Google
        url: https://www.google.com
        desc: 搜索引擎
        icon: assets/favicon.svg
        intranet: http://google.corp   # 可选：内网地址
        risk: medium                   # 可选：覆盖默认风险等级
        utm:                           # 可选：覆盖站点级 UTM
          source: nav
          medium: card
          campaign: homepage
```

- `site.color_scheme` 控制颜色模式（自动/明亮/深色），也可在网页右上角按钮切换并记忆。
- `site.theme_dir` 指向主题目录。主题目录需包含 `templates/` 与 `assets/`。
- `site.base_path` 可选：将站点输出到 `dist/<base_path>/`，部署后访问路径形如 `https://domain/<base_path>/index.html`。
- `icon` 可为相对路径或外链 URL。若不设置，也可不显示图标。
- `links[].intranet` 可选，配置后会在“内网版页面”使用该地址；未配置时会回退到外网地址。
- `site.redirect` 跳转页设置（仅外网模式生成）：
  - `delay_seconds` 自动跳转倒计时；为 0 或缺省时不自动跳转。
  - `default_risk` 默认风险等级（low|medium|high）。
  - `utm` 站点级 UTM 参数（source/medium/campaign/term/content）。
- `links[].risk` 可选：覆盖默认风险等级。
- `links[].utm` 可选：覆盖站点级 UTM 参数。

## 主题结构

- `templates/index.html.tera`：Tera 模板。可访问的变量：
  - `site_title`、`site_desc`、`color_scheme`（`auto|light|dark`）
  - `mode`（`external|intranet`）、`mode_other_label`（`外网|内网`）、`network_switch_href`
  - `groups`: 数组，每个元素包含 `name` 与 `links`
  - `links`: 每个链接包含 `name`、`href`、`desc`、`icon`、`host`（`href` 已按页面模式选择地址并自动回退）
- `templates/detail.html.tera`：单个链接的详情/跳转提示页模板（仅外网模式生成）。可访问变量：
  - `site_title`、`site_desc`、`color_scheme`
  - `link_name`、`link_desc`、`link_icon`、`link_host`、`link_url`
  - `risk_class`（low|medium|high）、`risk_label`（低/中/高风险）
  - `has_delay`（bool）、`delay_seconds`（数字）
- `assets/`：静态资源（CSS/JS/图标等），会复制到输出目录的 `assets/`。

### 输出说明

- `index.html` 外网版导航（若设置 `base_path`，在 `dist/<base_path>/index.html`）
- `intranet.html` 内网版导航（同上；若 `--no-intranet` 则不生成且页面不显示切换按钮）
- `go/<slug>/index.html` 每个链接的详情/跳转提示页（仅外网版生成；导航页会将链接指向这些中间页）

## 高级用法

```
cargo run --features remote -- build \
  --input-url https://gist.githubusercontent.com/<user>/<id>/raw/config.yaml \
  --out public --static static --theme themes/default --base-path secretPath --no-intranet

# 预览更多参数
cargo run -- preview --addr 127.0.0.1:9090 --dir dist/secretPath
# 或基于远程配置推导目录（需启用 remote 特性）：
cargo run --features remote -- preview --build-first --input-url https://gist.githubusercontent.com/<user>/<id>/raw/config.yaml
```

- `--input` 指定配置文件（默认自动寻找 `dove.yaml|dove.yml`）。
- `--out` 指定输出目录（默认 `dist/`）。
- `--static` 指定额外静态资源目录，递归拷贝到输出目录，可覆盖主题资源。
- `--theme` 指定主题目录，优先级高于 `site.theme_dir`。
- `--base-path` 指定站点根路径（相对子路径），优先级高于 `site.base_path`。
- `--no-intranet` 仅生成外网版本页面（不生成 `intranet.html`，且页面不显示切换按钮）。
- 预览命令（preview）：
  - `--build-first` 启动前先构建一次。
  - `--addr` 监听地址（默认 `127.0.0.1:8787`）。
  - `--dir` 指定服务目录（若未指定，将根据配置推导 `dist/<base_path>`）。

### 环境变量（可覆盖/提供与 CLI 同等的参数）

- `DOVE_INPUT`：配置文件路径（等价于 `--input`）
- `DOVE_INPUT_URL`：配置文件 URL（等价于 `--input-url`，需启用 `remote` 特性）
- `DOVE_GIST_URL`：配置文件 URL 的别名，指向 gist 的 raw 链接
- `DOVE_GIST_ID`：配置所在 gist 的 ID（将通过 GitHub API 获取 raw_url）
- `DOVE_GIST_FILE`：配合 `DOVE_GIST_ID` 指定文件名（不指定则取第一个文件）
- `DOVE_GITHUB_TOKEN`：访问私有 Gist 时的 token（会作为 `Authorization: token <TOKEN>` 加到请求头）
- `DOVE_AUTH_SCHEME`：可选，授权方案（默认 `token`，也可设为 `Bearer` 或其他值，最终头格式为 `Authorization: <SCHEME> <TOKEN>`）
- `DOVE_OUT`：输出目录（等价于 `--out`）
- `DOVE_PREVIEW_ADDR`：预览监听地址（等价于 `--addr`）
- `DOVE_STATIC`：静态资源目录（等价于 `--static`）
- `DOVE_THEME`：主题目录（等价于 `--theme`）
- `DOVE_THEME_DIR`：主题目录（`DOVE_THEME` 的别名）
- `DOVE_BASE_PATH`：站点根路径（等价于 `--base-path`）
- `DOVE_NO_INTRANET`：是否仅生成外网（真值如 `1/true/yes/on` 有效）
- `DOVE_COLOR_SCHEME`：页面配色方案覆盖（`auto|light|dark`）
- `DOVE_TITLE`：覆盖站点标题（仅影响渲染，不修改配置文件）
- `DOVE_DESCRIPTION`：覆盖站点描述（仅影响渲染，不修改配置文件）

优先级：CLI > 环境变量 > 配置文件 > 默认值。

私有 Gist 建议：使用 `DOVE_INPUT_URL` 指向 Gist 的 raw 链接，或设置 `DOVE_GIST_ID` 并提供 `DOVE_GITHUB_TOKEN`（可配合 `DOVE_AUTH_SCHEME=Bearer`）；两种方式都会在请求中自动携带 `Authorization` 头。以上需启用 `remote` 特性。

## 部署到 Cloudflare Pages

推荐两种方式：

1) 本地生成后上传
- 本地执行 `cargo run -- build` 生成 `dist/`。
- 在 Cloudflare Pages 新建项目，选择 “直接上传（Direct Upload）”，上传 `dist/` 目录内容。

2) 连接仓库并跳过构建
- 在 CI/CD 或本地生成 `dist/` 并提交到仓库。
- 在 Cloudflare Pages 连接该仓库，Framework 选择 None。
- Build command 留空或填 `echo skip`，Output directory 填 `dist`。

> 说明：Cloudflare Pages 默认并不会预装 Rust 工具链，不建议在 Pages 侧执行 `cargo` 构建。

## 许可

MIT
