"""启动工具箱网页及 PDF 缓存接口，需要 Node.js 22.12+。"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess


def main() -> int:
    parser = argparse.ArgumentParser(description="工具箱完整服务（网页 gzip 压缩及 PDF 缓存下载）")
    parser.add_argument("port", nargs="?", type=int, default=5173)
    parser.add_argument("--bind", default="0.0.0.0", help="监听地址，默认 0.0.0.0；仅本机访问可设为 127.0.0.1")
    parser.add_argument("--directory", default=str(Path(__file__).resolve().parent), help="构建后的网站目录")
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error("端口必须在 1～65535 之间")
    directory = Path(args.directory).resolve()
    entry = directory / "serve.mjs"
    if not entry.is_file() or not (directory / "server" / "index.mjs").is_file():
        parser.error("缺少配套后端，请重新构建并完整上传 dist（包含 server 目录）")
    node = shutil.which("node")
    if not node:
        parser.error("未找到 Node.js。请安装 Node.js 22.12+ 并加入 PATH 后重试")
    version = subprocess.run([node, "--version"], capture_output=True, text=True, check=True).stdout.strip()
    try:
        major, minor = map(int, version.lstrip("v").split(".")[:2])
    except ValueError:
        parser.error("无法识别 Node.js 版本：" + version)
    if (major, minor) < (22, 12):
        parser.error("需要 Node.js 22.12+，当前版本为 " + version)
    environment = os.environ.copy()
    environment.update(HOST=args.bind, PORT=str(args.port), TOOLBOX_STATIC_DIRECTORY=str(directory))
    if not environment.get("TOOLBOX_PDF_CACHE"):
        environment["TOOLBOX_PDF_CACHE"] = str(Path(__file__).resolve().parent / ".pdf-cache")
    with subprocess.Popen([node, str(entry)], env=environment) as process:
        try:
            return process.wait()
        except KeyboardInterrupt:
            if process.poll() is None:
                process.terminate()
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
