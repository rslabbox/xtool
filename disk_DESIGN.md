# xtool disk 设计文档

> 目的：定义 `xtool disk` 的命令层级、参数语义与行为规范，作为后续实现的唯一约定。本文档**不包含代码实现**。

## 设计目标

- 提供统一、可扩展的镜像文件操作接口（不直接操作物理磁盘）。
- 动作语义清晰（`mkimg`/`mkgpt`/`mkfs`/`ls`/`info`）。
- 以安全为优先：涉及破坏性操作时必须显式确认。
- 与现有 CLI 风格保持一致（动词子命令 + 位置参数 + 长短选项）。

## 顶层命令

```
xtool disk [global options] <action> [action options]
```

### 全局约定

- `--disk <PATH>`：目标镜像文件路径（例如 `disk.img`），所有子命令必需。
- `--part <ID|NAME>`：分区选择（可选），**名称优先自动解析为编号**，全局参数放在子命令前：
  1. 若为分区名（GPT label/partlabel），优先解析为分区编号；
  2. 若为纯数字，直接作为分区编号；
  3. 解析失败时输出可选分区列表并退出。
- 若未传 `--part`，则视为“无分区镜像”，子命令默认直接作用于**整个镜像文件**。
- 对镜像/分区写入类命令默认需要确认；脚本场景使用 `-y/--yes` 跳过交互确认。

> 注：`mkimg` 与 `mkgpt` 不依赖具体分区；`--part` 为可选全局参数，用于影响需要分区选择的子命令。

## 子命令清单

### 1) `mkimg` — 创建镜像文件

**用途**：创建指定大小的空镜像文件（仅文件，不触达物理磁盘）。

**参数**：
- `--disk <PATH>`：镜像文件路径（必填）
- `--size <SIZE>`：镜像大小（必填，支持 `K/M/G`）
- `--overwrite`：允许覆盖已有文件（默认否）

**行为说明**：
- 若目标文件已存在且未指定 `--overwrite`，应退出并提示。
- 镜像内容默认全 0 填充。

**示例**：
```
xtool disk --disk disk.img mkimg --size 64M
```

---

### 2) `mkgpt` — 生成 GPT 分区表

**用途**：根据 `parameter.txt` 创建 GPT 分区表。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `-f, --file <PATH>`：参数文件路径（必填，如 `parameter.txt`）
- `--align <SIZE>`：对齐大小（可选，默认 1M）
- `-y, --yes`：跳过确认（可选）

**行为说明**：
- 解析 `parameter.txt` 中 `CMDLINE` 字段（如 `mtdparts=...`）。
- 当参数文件无效或分区解析失败时，返回可读错误并退出。
- 若目标镜像已有分区表，必须二次确认或 `-y`。

**示例**：
```
xtool disk --disk disk.img mkgpt -f parameter.txt
```

---

### 3) `mkfs` — 格式化分区

**用途**：将指定分区格式化为目标文件系统。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析；为空则格式化整个 disk）
- `--fstype <TYPE>`：文件系统类型（必填，支持 `ext4`/`fat32`）
- `--label <LABEL>`：卷标（可选）
- `-y, --yes`：跳过确认（可选）

**行为说明**：
- `--part` 若为名称，优先解析为分区编号。
- 若 `--part` 解析失败，必须列出可选分区（编号 + 名称）。
- 该操作为破坏性操作，默认需二次确认。

**示例**：
```
xtool disk --disk disk.img --part root mkfs --fstype ext4
xtool disk --disk disk.img mkfs --fstype fat32 -y
```

---

### 4) `ls` — 列出分区文件

**用途**：列出指定分区内的文件与目录。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<PATH>`：列出指定目录（可选，默认 `/`）

**行为说明**：
- 只读操作，不修改分区内容。
- 若分区不存在或文件系统无法识别，返回明确错误。

**示例**：
```
xtool disk --disk disk.img --part root ls
xtool disk --disk disk.img ls /etc
```

---

### 5) `cp` — 复制文件（镜像内 ↔ 宿主机）

**用途**：在镜像分区与宿主机之间复制文件或目录。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<SRC>`：源路径（必填）
- `<DST>`：目标路径（必填）
- `-r, --recursive`：递归复制目录（可选）
- `-f, --force`：覆盖已存在目标（可选）
- `--preserve`：保留时间戳（可选）

**路径规则（宿主机交互）**：
- 使用 `host:` 前缀表示宿主机路径，例如 `host:/tmp/a.txt`。
- 未带 `host:` 前缀的路径视为镜像内路径（相对于分区根 `/`）。

**行为说明**：
- 允许 `host -> image`、`image -> host`、`image -> image` 三种方向。
- 若目标已存在且未指定 `--force`，应退出并提示。
- 目录复制必须显式指定 `-r`。

**示例**：
```
xtool disk --disk disk.img --part root cp host:/tmp/hello.txt /etc/hello.txt
xtool disk --disk disk.img --part root cp /etc/hello.txt host:/tmp/hello.txt
xtool disk --disk disk.img --part root cp -r /etc host:/tmp/etc
```

---

### 6) `mv` — 移动/重命名文件（镜像内 ↔ 宿主机）

**用途**：移动或重命名文件/目录，支持镜像分区与宿主机之间移动。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<SRC>`：源路径（必填）
- `<DST>`：目标路径（必填）
- `-f, --force`：覆盖已存在目标（可选）

**路径规则（宿主机交互）**：
- 使用 `host:` 前缀表示宿主机路径。
- 未带 `host:` 前缀的路径视为镜像内路径。

**行为说明**：
- `image -> image` 使用原子重命名（若文件系统支持）。
- `image <-> host` 以“复制 + 删除”实现，并要求确认（写入类）。
- 若跨设备移动失败，自动回退到复制+删除，并提示。

**示例**：
```
xtool disk --disk disk.img --part root mv /etc/old.conf /etc/new.conf
xtool disk --disk disk.img --part root mv host:/tmp/a.txt /etc/a.txt
xtool disk --disk disk.img --part root mv /etc/a.txt host:/tmp/a.txt
```

---

### 7) `rm` — 删除文件/目录（镜像内）

**用途**：删除镜像分区内的文件或目录。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<PATH>`：待删除路径（必填）
- `-r, --recursive`：递归删除目录（可选）
- `-f, --force`：忽略不存在的目标（可选）
- `-y, --yes`：跳过确认（可选）

**行为说明**：
- 仅作用于镜像内路径，不允许 `host:` 前缀。
- 删除目录需显式 `-r`。

**示例**：
```
xtool disk --disk disk.img --part root rm /etc/old.conf
xtool disk --disk disk.img --part root rm -r /var/log
```

---

### 8) `mkdir` — 创建目录（镜像内）

**用途**：在镜像分区内创建目录。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<PATH>`：目录路径（必填）
- `-p, --parents`：递归创建父目录（可选）

**行为说明**：
- 仅作用于镜像内路径，不允许 `host:` 前缀。

**示例**：
```
xtool disk --disk disk.img --part root mkdir /etc/app
xtool disk --disk disk.img --part root mkdir -p /var/lib/app/cache
```

---

### 9) `cat` — 查看文件内容（镜像内）

**用途**：输出镜像分区内文件内容到标准输出。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--part <ID|NAME>`：分区选择（可选，名称优先解析）
- `<PATH>`：文件路径（必填）
- `--bytes <N>`：仅读取前 N 字节（可选）
- `--offset <N>`：从偏移开始读取（可选）

**行为说明**：
- 仅作用于镜像内路径，不允许 `host:` 前缀。
- 二进制文件会直接输出原始字节。

**示例**：
```
xtool disk --disk disk.img --part root cat /etc/hostname
xtool disk --disk disk.img --part root cat /var/log/syslog --bytes 1024
```

---

### 10) `info` — 显示镜像与分区信息

**用途**：显示镜像布局、分区编号、名称、大小、文件系统类型。

**参数**：
- `--disk <PATH>`：目标镜像文件（必填）
- `--json`：结构化输出（可选）

**行为说明**：
- 输出分区列表，包含编号与名称，以便 `--part` 名称解析。

**示例**：
```
xtool disk --disk disk.img info
xtool disk --disk disk.img info --json
```

## 错误与提示规范

- 分区解析失败时，输出可选列表（编号/名称/大小）。
- 破坏性操作默认提示确认，明确影响范围。
- 参数缺失时，输出简短用法与示例。

## 与现有风格的对齐点

- 动词子命令风格，与 `tftpc get/put`、`file send/get` 对齐。
- 位置参数 + 明确长选项，减少歧义。
- 示例命令可直接写入 README 的“Usage”部分。
