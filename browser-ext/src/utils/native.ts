import { Application, Configuration, CoreApi } from "@goauthentik/api";

export interface Message {
    version: string;
    path: string;
    profile?: string;
    id: string;
    data: { [key: string]: unknown };
}

export interface Response {
    response_to: string;
    data: { [key: string]: unknown };
    error?: string;
}

const browserApi = (globalThis as typeof globalThis & { browser?: typeof chrome }).browser;
const runtimeApi = browserApi?.runtime ?? chrome.runtime;

function createRandomString(length: number = 16) {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let result = "";
    const randomArray = new Uint8Array(length);
    crypto.getRandomValues(randomArray);
    randomArray.forEach((number) => {
        result += chars[number % chars.length];
    });
    return result;
}

const defaultReconnectDelay = 5;
const requestTimeoutMs = 2500;

type PendingRequest = PromiseWithResolvers<Response> & {
    timeout?: ReturnType<typeof setTimeout>;
};

export class Native {
    #port?: chrome.runtime.Port;
    #promises: Map<string, PendingRequest> = new Map();
    #reconnectDelay = defaultReconnectDelay;
    #reconnectTimeout = 0;
    #isConnected = false;

    constructor() {
        this.#connect();
    }

    #connect() {
        const port = runtimeApi.connectNative("io.goauthentik.platform");
        this.#port = port;
        this.#isConnected = true;
        this.#reconnectDelay = defaultReconnectDelay;
        port.onMessage.addListener(this.#listener.bind(this));
        port.onDisconnect.addListener(() => {
            this.#isConnected = false;
            this.#reconnectDelay *= 1.35;
            this.#reconnectDelay = Math.min(this.#reconnectDelay, 3600);
            const err =
                (typeof chrome !== "undefined" ? chrome.runtime?.lastError : undefined) ||
                (port as chrome.runtime.Port & { error?: unknown }).error;
            this.#rejectPending(
                new Error(
                    `native host disconnected${err ? `: ${String(err)}` : ""}`,
                ),
            );
            this.#port = undefined;
            clearTimeout(this.#reconnectTimeout);
            this.#reconnectTimeout = setTimeout(() => {
                this.#connect();
            }, this.#reconnectDelay * 1000);
        });
    }

    #listener(msg: Response) {
        const prom = this.#promises.get(msg.response_to);
        if (!prom) {
            return;
        }
        if (msg.error) {
            if (prom.timeout) {
                clearTimeout(prom.timeout);
            }
            prom.reject(new Error(msg.error));
            this.#promises.delete(msg.response_to);
            return;
        }
        if (prom.timeout) {
            clearTimeout(prom.timeout);
        }
        prom.resolve(msg);
        this.#promises.delete(msg.response_to);
    }

    #postMessage(msg: Message, retry: boolean) {
        if (!this.#port || !this.#isConnected) {
            this.#connect();
        }
        if (!this.#port) {
            throw new Error("native host is not connected");
        }
        try {
            this.#port.postMessage(msg);
        } catch (exc) {
            const err = exc instanceof Error ? exc.message : String(exc);
            if (retry && err.includes("disconnected port")) {
                this.#isConnected = false;
                this.#port = undefined;
                this.#connect();
                this.#postMessage(msg, false);
                return;
            }
            throw exc;
        }
    }

    postMessage(msg: Partial<Message>): Promise<Response> {
        msg.id = createRandomString();
        const promise = Promise.withResolvers<Response>();
        try {
            const pending = promise as PendingRequest;
            pending.timeout = setTimeout(() => {
                this.#promises.delete(msg.id as string);
                pending.reject(
                    new Error(`native host timed out after ${requestTimeoutMs}ms`),
                );
            }, requestTimeoutMs);
            this.#promises.set(msg.id, pending);
            this.#postMessage(msg as Message, true);
        } catch (exc) {
            const pending = this.#promises.get(msg.id);
            if (pending?.timeout) {
                clearTimeout(pending.timeout);
            }
            pending?.reject(exc);
            this.#promises.delete(msg.id);
        }
        return promise.promise;
    }

    #rejectPending(error: Error) {
        for (const [id, pending] of this.#promises) {
            if (pending.timeout) {
                clearTimeout(pending.timeout);
            }
            pending.reject(error);
            this.#promises.delete(id);
        }
    }

    async ping(): Promise<Response> {
        return this.postMessage({
            version: "1",
            path: "ping",
        });
    }

    async getToken(profile: string): Promise<{ token: string; url: string }> {
        const token = await this.postMessage({
            version: "1",
            path: "get_token",
            profile: profile,
        });
        return {
            token: token.data.token as string,
            url: token.data.url as string,
        };
    }

    async listProfiles(): Promise<{ profiles: { name: string }[] }> {
        const profiles = await this.postMessage({
            version: "1",
            path: "list_profiles",
        });
        return {
            profiles: profiles.data.profiles as unknown as { name: string }[],
        };
    }

    async platformSignEndpointHeader(profile: string, challenge: string): Promise<string> {
        const response = await this.postMessage({
            version: "1",
            path: "platform_sign_endpoint_header",
            profile: profile,
            data: {
                challenge: challenge,
            },
        });
        return response.data.response as string;
    }

    async getApplications(profile: string): Promise<Application[]> {
        const token = await this.getToken(profile);

        try {
            const response = await new CoreApi(
                new Configuration({
                    basePath: `${token.url}/api/v3`,
                    accessToken: token.token,
                }),
            ).coreApplicationsList({});
            const apps = response.results.map((app) => {
                if (app.launchUrl && app.launchUrl.startsWith("/")) {
                    return { ...app, launchUrl: `${token.url}${app.launchUrl}` };
                }
                return app;
            });
            return apps;
        } catch (exc) {
            console.warn(`authentik/bext: failed to get applications: ${exc}`);
            return [];
        }
    }
}
