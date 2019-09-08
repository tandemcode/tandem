import {
  KeyValue,
  generateUID,
  EMPTY_ARRAY,
  EMPTY_OBJECT,
  getNestedTreeNodeById,
  memoize,
  TreeNode,
  updateNestedNode,
  findTreeNodeParent,
  arraySplice,
  findNestedNode,
  filterTreeNodeParents,
  containsNestedTreeNodeById
} from "tandem-common";
import { DependencyGraph } from "./graph";
import { last, uniq } from "lodash";
import {
  getPCNode,
  PCVisibleNode,
  getPCNodeContentNode,
  getPCNodeDependency,
  PCSourceTagNames,
  PCOverride,
  PCComponent,
  PCComponentInstanceElement,
  getOverrides,
  getPCVariantOverrides,
  extendsComponent,
  getPCNodeModule,
  PCModule,
  PCStyleMixin
} from "./dsl";
import { diffTreeNode, patchTreeNode } from "./ot";
import { SyntheticCSSStyleSheet } from "./synthetic-cssom";

/*------------------------------------------
 * STATE
 *-----------------------------------------*/

export const SYNTHETIC_DOCUMENT_NODE_NAME = "document";

export type SyntheticBaseNode = {
  metadata: KeyValue<any>;
  sourceNodeId: string;
} & TreeNode<string>;

export type SyntheticDocument = {
  instancePath?: string;
  children: SyntheticContentNode[];
} & SyntheticBaseNode;

export type SyntheticElement = {
  instancePath: string;
  className: string;
  attributes: KeyValue<string>;
  children: Array<SyntheticVisibleNode | PCOverride>;
} & SyntheticBaseNode;

export type SyntheticInstanceElement = {
  variant: KeyValue<boolean>;
  instancePath: string;
} & SyntheticElement;

export type SyntheticTextNode = {
  instancePath: string;
  className?: string;
  value: string;

  // DEPRECATED
  style: KeyValue<any>;
  children: Array<PCOverride>;
} & SyntheticBaseNode;

export type SyntheticVisibleNode = SyntheticElement | SyntheticTextNode;
export type SyntheticContentNode = {
  sheet?: SyntheticCSSStyleSheet;
} & SyntheticVisibleNode;
export type SyntheticNode = SyntheticDocument | SyntheticVisibleNode;

/*------------------------------------------
 * STATE FACTORIES
 *-----------------------------------------*/

export const createSytheticDocument = (
  sourceNodeId: string,
  children?: SyntheticVisibleNode[]
): SyntheticDocument => ({
  id: `synthetic-dom-${sourceNodeId}`,
  metadata: EMPTY_OBJECT,
  sourceNodeId,
  name: SYNTHETIC_DOCUMENT_NODE_NAME,
  children: children || EMPTY_ARRAY
});

/*------------------------------------------
 * TYPE UTILS
 *-----------------------------------------*/

export const isPaperclipState = (state: any) => Boolean(state.frames);

export const isSyntheticVisibleNodeRoot = (
  node: SyntheticVisibleNode,
  graph: DependencyGraph
) => getSyntheticSourceFrame(node, graph).children[0].id === node.sourceNodeId;

export const isSyntheticContentNode = (
  node: SyntheticVisibleNode,
  graph: DependencyGraph
) => {
  const sourceNode = getSyntheticSourceNode(node, graph);
  const module: PCModule = getPCNodeModule(sourceNode.id, graph);
  return module.children.indexOf(sourceNode) !== -1;
};

export const isSyntheticInstanceElement = (
  node: SyntheticNode
): node is SyntheticInstanceElement => {
  return Boolean((node as SyntheticInstanceElement).variant);
};

export const isSyntheticDocument = (
  node: SyntheticNode
): node is SyntheticDocument => {
  return node.name === SYNTHETIC_DOCUMENT_NODE_NAME;
};

export const isSyntheticElement = (
  node: SyntheticNode
): node is SyntheticElement => {
  return Boolean((node as SyntheticElement).attributes);
};

export const isSyntheticTextNode = (
  node: SyntheticNode
): node is SyntheticTextNode => {
  return (node as SyntheticTextNode).name === PCSourceTagNames.TEXT;
};

export const isSyntheticVisibleNode = (
  node: any
): node is SyntheticVisibleNode => {
  const sn = node as SyntheticVisibleNode;
  if (!sn) return false;
  return Boolean(sn.sourceNodeId) && Boolean(sn.name);
};

/*------------------------------------------
 * GETTERS
 *-----------------------------------------*/

export const getInheritedAndSelfOverrides = memoize(
  (
    instance: SyntheticElement,
    document: SyntheticDocument,
    graph: DependencyGraph,
    variantId?: string
  ): PCOverride[] => {
    const parents = filterTreeNodeParents(
      instance.id,
      document,
      () => true
    ) as SyntheticNode[];

    return parents.reduce((overrides: PCOverride[], parent: SyntheticNode) => {
      return [
        ...getOverrides(getSyntheticSourceNode(parent, graph)).filter(
          override =>
            override.variantId == variantId &&
            override.targetIdPath.indexOf(instance.sourceNodeId) !== -1
        ),
        ...overrides
      ];
    }, getPCVariantOverrides(getSyntheticSourceNode(instance, graph) as PCComponentInstanceElement, variantId));
  }
);

export const getSyntheticNodeStyleColors = memoize((node: SyntheticNode) => {
  console.error("TODO need to do this");
  return [];
});

// TODO - move to CSSOM
// export const _getSyntheticNodeStyleColors = memoize((node: SyntheticNode) => {
//   const colors: string[] = [];
//   if ((node as SyntheticVisibleNode).style) {
//     for (const key in (node as SyntheticVisibleNode).style) {
//       const value: string = (node as SyntheticVisibleNode).style[key];
//       const colorParts = String(value).match(/((rgba?|hsl)\(.*\)|#[^\s]+)/);
//       if (colorParts) {
//         colors.push(colorParts[1]);
//       }
//     }
//   }

//   for (let i = 0, { length } = node.children; i < length; i++) {
//     colors.push(
//       ..._getSyntheticNodeStyleColors(node.children[i] as SyntheticNode)
//     );
//   }

//   return colors;
// });

export const getSyntheticSourceNode = (
  node: SyntheticVisibleNode | SyntheticDocument,
  graph: DependencyGraph
) =>
  getPCNode(node.sourceNodeId, graph) as
    | PCVisibleNode
    | PCComponent
    | PCModule
    | PCStyleMixin;

export const getSyntheticSourceFrame = (
  node: SyntheticVisibleNode,
  graph: DependencyGraph
) =>
  getPCNodeContentNode(
    node.sourceNodeId,
    getPCNodeDependency(node.sourceNodeId, graph).content
  );

export const getSyntheticDocumentByDependencyUri = memoize(
  (
    uri: string,
    documents: SyntheticDocument[],
    graph: DependencyGraph
  ): SyntheticDocument => {
    return documents.find((document: SyntheticDocument) => {
      const dependency = getPCNodeDependency(document.sourceNodeId, graph);
      return dependency && dependency.uri === uri;
    });
  }
);

export const getSyntheticContentNode = memoize(
  (
    node: SyntheticVisibleNode,
    documentOrDocuments: SyntheticDocument | SyntheticDocument[]
  ) => {
    const documents = Array.isArray(documentOrDocuments)
      ? documentOrDocuments
      : [documentOrDocuments];
    const document = getSyntheticVisibleNodeDocument(node.id, documents);
    return document.children.find(
      contentNode =>
        contentNode.id === node.id ||
        containsNestedTreeNodeById(node.id, contentNode)
    );
  }
);

export const getSyntheticDocumentDependencyUri = (
  document: SyntheticDocument,
  graph: DependencyGraph
) => {
  return getPCNodeDependency(document.sourceNodeId, graph).uri;
};

export const getSyntheticVisibleNodeDocument = memoize(
  (
    syntheticNodeId: string,
    syntheticDocuments: SyntheticDocument[]
  ): SyntheticDocument => {
    return syntheticDocuments.find(document => {
      return containsNestedTreeNodeById(syntheticNodeId, document);
    });
  }
);

export const getSyntheticSourceUri = (
  syntheticNode: SyntheticVisibleNode,
  graph: DependencyGraph
) => {
  return getPCNodeDependency(syntheticNode.sourceNodeId, graph).uri;
};

export const getSyntheticNode = (
  node: SyntheticNode,
  documents: SyntheticDocument[]
) => getSyntheticNodeById(node.id, documents);

export const getSyntheticNodeById = memoize(
  (
    syntheticNodeId: string,
    documents: SyntheticDocument[]
  ): SyntheticVisibleNode => {
    const document = getSyntheticVisibleNodeDocument(
      syntheticNodeId,
      documents
    );
    if (!document) {
      return null;
    }
    return (getNestedTreeNodeById(
      syntheticNodeId,
      document
    ) as any) as SyntheticVisibleNode;
  }
);

export const getSyntheticNodeSourceDependency = (
  node: SyntheticVisibleNode | SyntheticDocument,
  graph: DependencyGraph
) => getPCNodeDependency(node.sourceNodeId, graph);

export const findInstanceOfPCNode = memoize(
  (
    node: PCVisibleNode | PCComponent,
    documents: SyntheticDocument[],
    graph: DependencyGraph
  ) => {
    for (const document of documents) {
      const instance = findNestedNode(document, (instance: SyntheticNode) => {
        return instance.sourceNodeId === node.id;
      });
      if (instance) {
        return instance;
      }
    }
    return null;
  }
);

export const findClosestParentComponentInstance = memoize(
  (
    node: SyntheticVisibleNode,
    root: SyntheticVisibleNode | SyntheticDocument,
    graph: DependencyGraph
  ) => {
    return findTreeNodeParent(node.id, root, (parent: SyntheticVisibleNode) => {
      return isComponentOrInstance(parent, graph);
    });
  }
);

export const findFurthestParentComponentInstance = memoize(
  (
    node: SyntheticVisibleNode,
    root: SyntheticVisibleNode | SyntheticDocument,
    graph: DependencyGraph
  ) => {
    const parentComponentInstances = getAllParentComponentInstance(
      node,
      root,
      graph
    );
    return parentComponentInstances.length
      ? parentComponentInstances[parentComponentInstances.length - 1]
      : null;
  }
);

export const getAllParentComponentInstance = memoize(
  (
    node: SyntheticVisibleNode,
    root: SyntheticVisibleNode | SyntheticDocument,
    graph: DependencyGraph
  ) => {
    let current = findClosestParentComponentInstance(node, root, graph);
    if (!current) return [];
    const instances = [current];
    while (current) {
      const parent = findClosestParentComponentInstance(current, root, graph);
      if (!parent) break;
      current = parent;

      instances.push(current);
    }

    return instances;
  }
);

export const isComponentOrInstance = (
  node: SyntheticVisibleNode,
  graph: DependencyGraph
) => {
  const sourceNode = getSyntheticSourceNode(node, graph);

  // source node may have been deleted, so return false is that's the case

  if (!sourceNode) {
    return false;
  }
  return (
    sourceNode.name === PCSourceTagNames.COMPONENT ||
    sourceNode.name === PCSourceTagNames.COMPONENT_INSTANCE
  );
};

export const getNearestComponentInstances = memoize(
  (
    node: SyntheticVisibleNode,
    root: SyntheticVisibleNode | SyntheticDocument,
    graph: DependencyGraph
  ) => {
    const instances = getAllParentComponentInstance(node, root, graph);
    if (isComponentOrInstance(node, graph)) {
      return [node, ...instances];
    }
    return instances;
  }
);

export const getSyntheticInstancePath = memoize(
  (
    node: SyntheticNode,
    root: SyntheticVisibleNode | SyntheticDocument,
    graph: DependencyGraph
  ) => {
    const nodePath = getAllParentComponentInstance(
      node as SyntheticVisibleNode,
      root,
      graph
    ).reduce(
      (nodePath: string[], instance: SyntheticInstanceElement) => {
        const lastId = last(nodePath);
        const currentSourceNode = getSyntheticSourceNode(
          instance,
          graph
        ) as PCComponentInstanceElement;

        let current:
          | PCComponent
          | PCComponentInstanceElement = currentSourceNode;

        while (current && extendsComponent(current)) {
          current = getPCNode(current.is, graph) as PCComponent;
          if (containsNestedTreeNodeById(lastId, current)) {
            return [...nodePath, currentSourceNode.id];
          }
        }

        return nodePath;
      },
      [node.sourceNodeId]
    );

    // only want instance path, so strip initial source node ID
    return nodePath.slice(1).reverse();
  }
);

export type SyntheticInstanceMap = {
  [identifier: string]: string[];
};

export const getSyntheticSourceMap = memoize(
  (current: SyntheticVisibleNode | SyntheticDocument) => {
    return Object.assign({}, ...getSyntheticSourceFlatMap(current));
  }
);

export const getSyntheticDocumentsSourceMap = memoize(
  (documents: SyntheticDocument[]) => {
    const flatMap = [];
    for (let i = 0, { length } = documents; i < length; i++) {
      flatMap.push(getSyntheticSourceMap(documents[i]));
    }
    return Object.assign({}, ...flatMap);
  }
);

const getSyntheticSourceFlatMap = memoize(
  (current: SyntheticVisibleNode | SyntheticDocument) => {
    const path = current.instancePath
      ? current.instancePath + "." + current.sourceNodeId
      : current.sourceNodeId;
    const map = { [path]: current.id };
    const flatMap = [map];
    for (let i = 0, { length } = current.children; i < length; i++) {
      flatMap.push(
        ...getSyntheticSourceFlatMap(current.children[
          i
        ] as SyntheticVisibleNode)
      );
    }

    return flatMap;
  }
);

export const syntheticNodeIsInShadow = (
  node: SyntheticNode,
  root: SyntheticVisibleNode | SyntheticDocument,
  graph: DependencyGraph
) => getSyntheticInstancePath(node, root, graph).length > 0;

// alias
export const isSyntheticNodeImmutable = syntheticNodeIsInShadow;

export const stringifySyntheticNode = memoize(
  (node: SyntheticNode, depth: number = 0) => {
    const indent = "  ".repeat(depth);
    if (isSyntheticTextNode(node)) {
      if (!node.className) {
        return indent + node.value + "\n";
      }
      return indent + `<span class="${node.className}">${node.value}</span>\n`;
    } else if (isSyntheticElement(node)) {
      let buffer = `${indent}<${node.name}`;
      if (node.className) {
        buffer += ` class="${node.className}"`;
      }
      for (const key in node.attributes) {
        buffer += ` ${key}="${node.attributes[key]}"`;
      }
      buffer += `>\n`;
      for (const child of node.children) {
        buffer += stringifySyntheticNode(child, depth + 1);
      }

      buffer += `${indent}</${node.name}>\n`;
      return buffer;
    } else {
      return node.children
        .map(child => stringifySyntheticNode(child, depth))
        .join("\n");
    }
  }
);

/*------------------------------------------
 * SETTERS
 *-----------------------------------------*/

export const upsertSyntheticDocument = (
  newDocument: SyntheticDocument,
  oldDocuments: SyntheticDocument[],
  graph: DependencyGraph
) => {
  const oldDocumentIndex = oldDocuments.findIndex(
    oldDocument => oldDocument.sourceNodeId === newDocument.sourceNodeId
  );
  if (oldDocumentIndex === -1) {
    return [...oldDocuments, newDocument];
  }
  const oldDocument = oldDocuments[oldDocumentIndex];
  return arraySplice(
    oldDocuments,
    oldDocumentIndex,
    1,
    patchTreeNode(diffTreeNode(oldDocument, newDocument), oldDocument)
  );
};

export const updateSyntheticVisibleNodeMetadata = (
  metadata: KeyValue<any>,
  node: SyntheticVisibleNode,
  document: SyntheticDocument
) =>
  updateNestedNode(node, document, node => ({
    ...node,
    metadata: {
      ...node.metadata,
      ...metadata
    }
  }));
