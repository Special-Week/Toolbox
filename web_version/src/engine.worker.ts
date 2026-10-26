/// <reference lib="webworker" />
/** 独立 Worker 承载 Wasm，所有计算均不占用浏览器界面线程。 */
import init, {
  catalog_json,
  gauge_page,
  calculate,
  spline_chunk,
  convert_pressure,
} from "../pkg/toolbox_core";
const ready = init();
self.onmessage = async (event: MessageEvent) => {
  const { id, method, payload } = event.data;
  try {
    await ready;
    let result: unknown;
    if (method === "catalog") result = JSON.parse(catalog_json());
    else if (method === "calculate")
      result = JSON.parse(
        calculate(payload.tool, JSON.stringify(payload.input)),
      );
    else if (method === "gaugePage")
      result = JSON.parse(
        gauge_page(
          payload.tool,
          JSON.stringify(payload.input),
          payload.changed ?? "",
        ),
      );
    else if (method === "splineChunk")
      result = spline_chunk(
        JSON.stringify(payload.input),
        payload.offset,
        payload.limit,
      );
    else if (method === "convertPressure")
      result = JSON.parse(
        convert_pressure(
          JSON.stringify(payload.values),
          payload.source,
          payload.target,
        ),
      );
    else throw new Error("未知计算请求");
    self.postMessage({ id, result });
  } catch (error) {
    self.postMessage({
      id,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
