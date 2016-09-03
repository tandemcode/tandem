import { IActor } from "sf-common/actors";
import { find, ActiveRecord, IFile } from "sf-common/active-records";
import {
  Dependencies,
  MainBusDependency,
  MimeTypeDependency,
  ActiveRecordFactoryDependency,
} from "sf-common/dependencies";

import { bindable } from "sf-common/decorators";

export const FILES_COLLECTION_NAME = "files";

export class File extends ActiveRecord implements IFile {

  @bindable()
  public path: string;

  @bindable()
  public content: string;

  readonly type: string;
  serialize() {
    return {
      path: this.path,
      content: this.content
    };
  }

  static findAll(dependencies: Dependencies) {
    return this.find(undefined, dependencies);
  }

  static create(sourceData: any, dependencies: Dependencies) {
    const activeRecordFactory = ActiveRecordFactoryDependency.find(MimeTypeDependency.lookup(sourceData.path, dependencies), dependencies) || ActiveRecordFactoryDependency.find("file", dependencies);
    return activeRecordFactory.create(FILES_COLLECTION_NAME, sourceData);
  }

  static async find(query: any, dependencies: Dependencies): Promise<Array<File>> {

    // TODO FILES should not be here
    return (await find(FILES_COLLECTION_NAME, query, true, dependencies)).map((sourceData) => {
      return File.create(sourceData, dependencies);
    });
  }
}

export const fileModelDependency = new ActiveRecordFactoryDependency("file", File);