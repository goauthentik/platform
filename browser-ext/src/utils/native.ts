export interface Message {
    version: string;
    path: string;
    profile: string;
    id: string;
}

export interface Response {
    response_to: string;
}

export class Native {

    #port?: chrome.runtime.Port;
    #promises: Map<string, PromiseWithResolvers<Response>> = new Map();

    constructor() {
        this.#connect();
    }

    #connect() {
        this.#port = chrome.runtime.connectNative('io.goauthentik.agent');
        this.#port.onMessage.addListener(this.#listener.bind(this));
        this.#port.onDisconnect.addListener(() => {
            console.log('Disconnected, reconnecting');
            this.#connect();
        });
        console.debug("Connected to native")
    }

    #listener(msg: Response) {
        const prom = this.#promises.get(msg.response_to);
        console.debug("Got response", msg);
        if (!prom) {
            console.debug("No promise to resolve");
            return
        }
        prom.resolve(msg);
    }

    postMessage(msg: Message): Promise<Response> {
        const promise = Promise.withResolvers<Response>();
        this.#promises.set(msg.id, promise);
        console.debug(`Sending message`, msg);
        this.#port?.postMessage(msg);
        return promise.promise;
    }
}
