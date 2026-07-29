/**
 * 本地静态服务器（bun 版），带 COEP/COOP 头，用于在本机浏览器打开 GPUI 的 Web 产物。
 *
 * 为什么不用 Python：Windows 默认没有 Python，需自行安装；而 Web 学习者大多已有
 * bun/node。bun 是单二进制、零额外依赖，直接用 `bun run serve.ts` 即可。
 *
 * 为什么需要 COEP/COOP：WebGPU 与 SharedArrayBuffer 的硬性要求，否则浏览器会拒绝
 * 执行 wasm 里的相关特性。直接 file:// 打开或普通静态服务器都跑不起来。
 *
 * 用法：
 *   bun run serve.ts [port] [dir]      # 默认 port=8001, dir=当前工作目录
 * 然后浏览器打开 http://127.0.0.1:8001/
 * （build_web.sh --serve 会先 cd 进 web_dist 再调本脚本，故默认托管 web_dist。）
 */
const port = Number(Bun.argv[2] ?? "8001");
// 默认托管当前工作目录（build_web.sh --serve 会先 cd 进 web_dist 再调本脚本，
// 所以 cwd 就是产物目录；也可传第三个参数显式指定目录）。
const baseDir = Bun.argv[3] ?? process.cwd();

// 常见 MIME，避免浏览器把 .wasm/.js 当成错误类型。
const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
};

const server = Bun.serve({
  port,
  // 每个响应都带上跨源隔离所需的两个头（参考 zed xtask 的 web_examples 做法）。
  fetch: async (req) => {
    const url = new URL(req.url);
    let pathname = decodeURIComponent(url.pathname);
    if (pathname === "/") pathname = "/index.html";

    const file = Bun.file(`${baseDir}${pathname}`);
    if (!(await file.exists())) {
      return new Response("404 Not Found", { status: 404 });
    }
    const ext = pathname.slice(pathname.lastIndexOf("."));
    return new Response(file, {
      headers: {
        "Content-Type": MIME[ext] ?? "application/octet-stream",
        // 跨源隔离：WebGPU / SharedArrayBuffer 必需。
        "Cross-Origin-Embedder-Policy": "require-corp",
        "Cross-Origin-Opener-Policy": "same-origin",
      },
    });
  },
});

console.log(`serving ${baseDir} at http://127.0.0.1:${port}/  (Ctrl+C 退出)`);
