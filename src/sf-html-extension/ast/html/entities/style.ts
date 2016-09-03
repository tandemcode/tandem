import { inject } from "sf-common/decorators";
import { parseCSS } from "sf-html-extension/ast";
import { GroupNodeSection } from "sf-html-extension/dom";
import { HTMLElementEntity } from "./element";
import { EntityFactoryDependency } from "sf-common/dependencies";
import { CSSStyleSheetExpression } from "sf-html-extension/ast";
import { CSSStylesheetsDependency } from "sf-html-extension/dependencies";
import { HTMLElementExpression, HTMLTextExpression } from "sf-html-extension/ast";

export class HTMLStyleEntity extends HTMLElementEntity {
  async load() {
    super.load();
    const nodeValue = (<HTMLTextExpression>this.source.children[0]).value;
    CSSStylesheetsDependency.getInstance(this._dependencies).addStyleSheet(parseCSS(nodeValue));
  }

  createSection() {
    return new GroupNodeSection();
  }
}

export const htmlStyleEntityDependency = new EntityFactoryDependency(HTMLElementExpression, HTMLStyleEntity, "style");