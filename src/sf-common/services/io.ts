
import { Logger } from "sf-core/logger";
import { loggable, isPublic, document } from "sf-core/decorators";
import * as SocketIOBus from "mesh-socket-io-bus";

import { Service } from "sf-core/services";
import { ParallelBus, AcceptBus } from "mesh";
import * as sift from "sift";

import { IApplication } from "sf-core/application";
import { BaseApplicationService } from "sf-core/services";
import { Dependencies, Injector } from "sf-core/dependencies";

@loggable()
export default class IOService<T extends IApplication> extends BaseApplicationService<T> {

  public logger: Logger;
  public _publicActionTypes: any;
  public _remoteActors: Array<any>;

  load() {

    // this is the public service which handles all
    // incomming actions
    this._publicActionTypes = {};

    // scan the application for all public actions and add
    // then to the public service
    for (const actor of this.app.bus.actors) {
      for (const actionType of ((actor as any).__publicProperties || [])) {
        this.logger.info(`exposing ${actor.constructor.name}.${actionType}`);
        this._publicActionTypes[actionType] = true;
      }
    }

    // remote actors which take actions from the server
    this._remoteActors = [];

    // add the remote actors to the application so that they
    // receive actions from other parts of the application
    this.app.bus.register(ParallelBus.create(this._remoteActors));
  }

  /**
   * returns the publicly accessible actors
   */

  @isPublic
  @document("returns the public action types")
  getPublicActionTypes() {
    return Object.keys(this._publicActionTypes);
  }

  /**
   */

  @isPublic
  @document("pings remote connections")
  ping() {
    return "pong";
  }

  /**
   */

  @document("returns the number of remote connections")
  getRemoteConnectionCount() {
    return this._remoteActors.length;
  }

  /**
   */

  addConnection = async (connection) => {
    this.logger.info("client connected");

    const remoteService = new Service();

    // from here on, all global actions will touch on this remote service object.
    // If the action is registered to the service, that action will be executed
    // against the remote client.
    this._remoteActors.push(remoteService);

    // setup the bus which will facilitate in all
    // transactions between the remote service
    const remoteBus = SocketIOBus.create({
      connection: connection
    }, {
      execute: (action) => {
        action.remote = true;
        return this.bus.execute(action);
      }
    });

    // fetch the remote action types, and set them to the remote service
    // so that we limit the number of outbound actions
    for (const remoteActionType of await remoteBus.execute({ type: "getPublicActionTypes" }).readAll()) {
      this.logger.verbose("adding remote action \"%s\"", remoteActionType);
      remoteService.addActor(remoteActionType, new AcceptBus(sift({ remote: { $ne: true }}), remoteBus, null));
    }

    connection.once("disconnect", () => {
      this.logger.info("client disconnected");

      this._remoteActors.splice(
        this._remoteActors.indexOf(remoteService),
        1
      );
    });
  }

  static create<T extends IApplication>(dependencies: Dependencies): IOService<T> {
    return Injector.inject(new IOService<T>(), dependencies);
  }
}