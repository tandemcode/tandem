import { BaseComponent } from 'paperclip';

export default class HTML extends BaseComponent {
  update() {
    this.section.removeChildNodes();
    var node = this.attributes.value;
    if (node) {
      this.section.appendChild(node);
    }
  }
}
