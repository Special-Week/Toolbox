/** 请求编号与界面版本分开处理，过期结果不会覆盖用户的新输入。 */
export class Engine {
  private worker = new Worker(new URL("./engine.worker.ts", import.meta.url), {
    type: "module",
  });
  private pending = new Map<
    number,
    { resolve: (value: any) => void; reject: (reason: Error) => void }
  >();
  private serial = 0;
  constructor() {
    this.worker.onmessage = ({ data }) => {
      const p = this.pending.get(data.id);
      if (!p) return;
      this.pending.delete(data.id);
      if (data.error) p.reject(new Error(data.error));
      else p.resolve(data.result);
    };
    this.worker.onerror = () => {
      for (const p of this.pending.values())
        p.reject(
          new Error(
            "计算模块加载失败，请刷新页面，或确认浏览器支持 WebAssembly。",
          ),
        );
      this.pending.clear();
    };
  }
  request<T>(method: string, payload: unknown = {}): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = ++this.serial;
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ id, method, payload });
    });
  }
  close() {
    this.worker.terminate();
    for (const p of this.pending.values())
      p.reject(new Error("计算模块已关闭"));
    this.pending.clear();
  }
}
