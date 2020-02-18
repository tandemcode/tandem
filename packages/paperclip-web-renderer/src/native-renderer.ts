import { Html5Entities } from "html-entities";
import { stringifyCSSSheet } from "paperclip/lib/stringify-sheet";

const entities = new Html5Entities();

export type DOMNodeMap = Map<Node, string>;

export const createNativeNode = (
  node,
  protocol: string | null,
  map: DOMNodeMap = new Map()
) => {
  // if (!node || true) {
  // return document.createTextNode(JSON.stringify(node));
  // }
  try {
    switch (node.kind) {
      case "Text": {
        const text = createNativeTextNode(node);
        map.set(text, node.id);
        return text;
      }
      case "Element":
        return createNativeElement(node, protocol, map);
      case "StyleElement":
        return createNativeStyle(node, protocol, map);
      case "Fragment":
        return createNativeFragment(node, protocol, map);
    }
  } catch (e) {
    return document.createTextNode(String(e.stack));
  }
};

const createNativeTextNode = node => {
  return document.createTextNode(entities.decode(node.value));
};

const createNativeStyle = (element, protocol: string, map: DOMNodeMap) => {
  // return document.createTextNode(stringifyCSSSheet(element.sheet, protocol));
  const nativeElement = document.createElement("style");
  nativeElement.textContent = stringifyCSSSheet(element.sheet, protocol);
  map.set(nativeElement, element.id);
  return nativeElement;
};

const createNativeElement = (element, protocol: string, map: DOMNodeMap) => {
  const nativeElement = document.createElement(element.tagName);
  for (let { name, value } of element.attributes) {
    if (name === "src" && protocol) {
      value = value.replace(/\w+:/, protocol);
    }

    nativeElement.setAttribute(name, value);
  }
  for (const child of element.children) {
    nativeElement.appendChild(createNativeNode(child, protocol, map));
  }
  map.set(nativeElement, element.id);
  return nativeElement;
};

const createNativeFragment = (fragment, protocol: string, map: DOMNodeMap) => {
  const nativeFragment = document.createDocumentFragment();
  for (const child of fragment.children) {
    nativeFragment.appendChild(createNativeNode(child, protocol, map));
  }
  return nativeFragment;
};
