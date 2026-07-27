import assert from "node:assert/strict";
import test from "node:test";

import { readLcscItemImage } from "../src/lcsc/image.ts";

// C53309018 使用品牌证书目录作为商品图，必须与普通商品目录同样允许直连。
const controlledUrl =
  "https://alimg.szlcsc.com/upload/public/brand/product/certificate/20240701/example.jpg";

test("reads a controlled CORS image with matching MIME and signature", async (context) => {
  const originalFetch = globalThis.fetch;
  context.after(() => {
    globalThis.fetch = originalFetch;
  });
  globalThis.fetch = async (url, options) => {
    assert.equal(url, controlledUrl);
    assert.equal(options.credentials, "omit");
    assert.equal(options.redirect, "error");
    assert.equal(options.headers, undefined);
    return new Response(new Uint8Array([0xff, 0xd8, 0xff, 0x00]), {
      status: 200,
      headers: { "content-type": "image/jpeg", "content-length": "4" },
    });
  };

  const blob = await readLcscItemImage(controlledUrl);
  assert.equal(blob.type, "image/jpeg");
  assert.equal(blob.size, 4);
});

test("rejects untrusted URLs before issuing a request", async (context) => {
  const originalFetch = globalThis.fetch;
  context.after(() => {
    globalThis.fetch = originalFetch;
  });
  globalThis.fetch = async () => {
    throw new Error("fetch must not run");
  };

  await assert.rejects(
    readLcscItemImage("https://example.com/upload/public/product/a.jpg"),
    /图片地址无效/,
  );
  await assert.rejects(
    readLcscItemImage("https://alimg.szlcsc.com/upload/public/brand/private/a.jpg"),
    /图片地址无效/,
  );
});

test("rejects mismatched image content", async (context) => {
  const originalFetch = globalThis.fetch;
  context.after(() => {
    globalThis.fetch = originalFetch;
  });
  globalThis.fetch = async () =>
    new Response(new TextEncoder().encode("not-an-image"), {
      status: 200,
      headers: { "content-type": "image/jpeg" },
    });

  await assert.rejects(readLcscItemImage(controlledUrl), /图片内容无效/);
});
