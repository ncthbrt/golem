((global) => {
    const OPS_CACHE /*: { [name: string]: number } */ = Golem.core.ops();

    class Deferral {
        constructor() {
            this.done = false;
            this.reject = undefined;
            this.resolve = undefined;
            this.promise = new Promise((resolve, reject) => {
                this.reject = reject;
                this.resolve = resolve;
            });

            this.promise.then(() => {
                this.done = true;
            }).catch(() => {
                this.done = true;
            });
        }
    }

    function assert(value, log) {
        if (!value) {
            throw new Error(log ?? "Expected value to be truthy, but got false");
        }
    }

// Using an object without a prototype because `Map` was causing GC problems.
//     {
//         [key: number]: util.Resolvable<JsonResponse>;
//     }
    const promiseTable = Object.create(null);
    let _nextPromiseId = 1;

    function nextPromiseId() {
        return _nextPromiseId++;
    }

    function decode(ui8 /*: Uint8Array */) {
        console.log('calling decoder', ui8);
        const s = Golem.core.decode(ui8);
        return JSON.parse(s);
    }

    function encode(args) /*: Uint8Array */ {
        const s = JSON.stringify(args);
        return Golem.core.encode(s);
    }

    function unwrapResponse(res) {
        if (res.err != null) {
            throw new Error(res.err.message);
        }
        assert(res.ok != null);
        return res.ok;
    }

    function asyncMsgFromRust(resUi8 /*: Uint8Array */) {
        console.log('received asyncMsgFromRust', resUi8);
        const res = decode(resUi8);
        assert(res.promiseId != null);

        const promise = promiseTable[res.promiseId];
        assert(promise != null);
        delete promiseTable[res.promiseId];
        promise.resolve(res);
    }

    function sendSync(
        opName,
        args = {},
        zeroCopy // Uint8Array
    ) {
        const opId = OPS_CACHE[opName];
        console.log("sendSync", opName, opId);
        const argsUi8 = encode(args);
        const resUi8 = Golem.core.dispatch(opId, argsUi8, zeroCopy);
        assert(resUi8 != null);

        const res = decode(resUi8);
        assert(res.promiseId == null);
        return unwrapResponse(res);
    }

    async function sendAsync(
        opName,
        args = {},
        zeroCopy // Uint8Array
    ) {
        const opId = OPS_CACHE[opName];
        console.log("sendAsync", opName, opId);
        const promiseId = nextPromiseId();
        args = Object.assign(args, {promiseId});
        const promise = new Deferral();

        const argsUi8 = encode(args);
        const buf = Golem.core.dispatch(opId, argsUi8, zeroCopy);
        if (buf) {
            // Sync result.
            const res = decode(buf);
            promise.resolve(res);
        } else {
            // Async result.
            promiseTable[promiseId] = promise;
        }

        const res = await promise.promise;
        return unwrapResponse(res);
    }

    Golem.core.sendAsync = sendAsync;
    Golem.core.sendSync = sendSync;
    Golem.core.asyncMsgFromRust = asyncMsgFromRust;
    Golem.core.OPS_CACHE = OPS_CACHE;


})(this);
