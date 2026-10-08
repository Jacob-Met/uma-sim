import React, { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { SkillsPanel } from "/tmp/hamon-product-d98d06fb-20261008/uma-sim/packages/uma-sim-ui/src/components/SkillsPanel.tsx";
const review = window.__review = { requests: [], events: [], inputs: [] };
window.fetch = (url, options = {}) => {
  const index = review.requests.length;
  let resolveJson;
  const body = new Promise(resolve => { resolveJson = resolve; });
  const item = { index, url: String(url), method: options.method ?? "GET", hasBody: options.body !== undefined, signal: options.signal, resolveJson, jsonCalled: false };
  review.requests.push(item); review.events.push({event:"response-fulfilled", index});
  options.signal?.addEventListener("abort", () => review.events.push({event:"abort", index}));
  return Promise.resolve({ok:true, json: () => {item.jsonCalled = true; review.events.push({event:"json-called", index}); return body;}});
};
const root = createRoot(document.getElementById("root"));
review.render = (id) => {
 const state = Object.freeze({learnedSkillIds:Object.freeze([id]), hintLevels:Object.freeze({[id]:2})});
 review.inputs.push({state, before:JSON.stringify(state)});
 root.render(<StrictMode><SkillsPanel state={state as any} /></StrictMode>);
};
review.unmount = () => root.unmount();
review.render("skill:old");