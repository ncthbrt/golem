// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
/*
SharedQueue Binary Layout
+-------------------------------+-------------------------------+
|                        NUM_RECORDS (32)                       |
+---------------------------------------------------------------+
|                        NUM_SHIFTED_OFF (32)                   |
+---------------------------------------------------------------+
|                        HEAD (32)                              |
+---------------------------------------------------------------+
|                        OFFSETS (32)                           |
+---------------------------------------------------------------+
|                        RECORD_ENDS (*MAX_RECORDS)           ...
+---------------------------------------------------------------+
|                        RECORDS (*MAX_RECORDS)               ...
+---------------------------------------------------------------+
 */

/* eslint-disable @typescript-eslint/no-use-before-define */

((window) => {
    const GLOBAL_NAMESPACE = "Golem";
    const CORE_NAMESPACE = "core";
    const MAX_RECORDS = 100;
    const INDEX_NUM_RECORDS = 0;
    const INDEX_NUM_SHIFTED_OFF = 1;
    const INDEX_HEAD = 2;
    const INDEX_OFFSETS = 3;
    const INDEX_RECORDS = INDEX_OFFSETS + 2 * MAX_RECORDS;
    const HEAD_INIT = 4 * INDEX_RECORDS;

    // Available on start due to bindings.
    const Golem = window[GLOBAL_NAMESPACE];
    const core = Golem[CORE_NAMESPACE];
    // Warning: DO NOT use window.Golem after this point.
    // It is possible that the Golem namespace has been deleted.
    // Use the above local Golem and core variable instead.

    let sharedBytes;
    let shared32;

    let asyncHandlers;

    let initialized = false;

    function maybeInit() {
        if (!initialized) {
            init();
            initialized = true;
        }
    }

    function init() {
        const shared = Golem.core.shared;
        assert(shared.byteLength > 0, "Expected shared byteLength to be greater than zero");
        assert(sharedBytes == null, "Expected sharedBytes to be nonnull");
        assert(shared32 == null, "Expected shared32 to be nonnull");
        sharedBytes = new Uint8Array(shared);
        shared32 = new Int32Array(shared);
        asyncHandlers = [];
        // Callers should not call Golem.core.recv, use setAsyncHandler.
        Golem.core.recv(handleAsyncMsgFromRust);
    }

    function ops() {
        // op id 0 is a special value to retrieve the map of registered ops.
        const opsMapBytes = Golem.core.send(0, new Uint8Array([]), null);
        const opsMapJson = String.fromCharCode.apply(null, opsMapBytes);
        return JSON.parse(opsMapJson);
    }

    function assert(cond, msg) {
        if (!cond) {
            throw Error(msg ?? "Expected condition to be true");
        }
    }

    function reset() {
        maybeInit();
        shared32[INDEX_NUM_RECORDS] = 0;
        shared32[INDEX_NUM_SHIFTED_OFF] = 0;
        shared32[INDEX_HEAD] = HEAD_INIT;
    }

    function head() {
        maybeInit();
        return shared32[INDEX_HEAD];
    }

    function numRecords() {
        return shared32[INDEX_NUM_RECORDS];
    }

    function size() {
        return shared32[INDEX_NUM_RECORDS] - shared32[INDEX_NUM_SHIFTED_OFF];
    }

    function setMeta(index, end, opId) {
        shared32[INDEX_OFFSETS + 2 * index] = end;
        shared32[INDEX_OFFSETS + 2 * index + 1] = opId;
    }

    function getMeta(index) {
        if (index < numRecords()) {
            const buf = shared32[INDEX_OFFSETS + 2 * index];
            const opId = shared32[INDEX_OFFSETS + 2 * index + 1];
            return [opId, buf];
        } else {
            return null;
        }
    }

    function getOffset(index) {
        if (index < numRecords()) {
            if (index == 0) {
                return HEAD_INIT;
            } else {
                const prevEnd = shared32[INDEX_OFFSETS + 2 * (index - 1)];
                return (prevEnd + 3) & ~3;
            }
        } else {
            return null;
        }
    }

    function push(opId, buf) {
        const off = head();
        const end = off + buf.byteLength;
        const alignedEnd = (end + 3) & ~3;
        const index = numRecords();
        if (alignedEnd > shared32.byteLength || index >= MAX_RECORDS) {
            // console.log("shared_queue.js push fail");
            return false;
        }
        setMeta(index, end, opId);
        assert(alignedEnd % 4 === 0, "Expected to be aligned");
        assert(end - off == buf.byteLength, "Expected to be aligned 2");
        sharedBytes.set(buf, off);
        shared32[INDEX_NUM_RECORDS] += 1;
        shared32[INDEX_HEAD] = alignedEnd;
        return true;
    }

    /// Returns null if empty.
    function shift() {
        const i = shared32[INDEX_NUM_SHIFTED_OFF];
        if (size() == 0) {
            assert(i == 0, "Expected i to be zero");
            return null;
        }

        const off = getOffset(i);
        const [opId, end] = getMeta(i);

        if (size() > 1) {
            shared32[INDEX_NUM_SHIFTED_OFF] += 1;
        } else {
            reset();
        }

        assert(off != null, "Expected off to be non null");
        assert(end != null, "Expected end to be non null");
        const buf = sharedBytes.subarray(off, end);
        return [opId, buf];
    }

    function setAsyncHandler(opId, cb) {
        maybeInit();
        assert(opId != null, "Expected opId to be non-null");
        asyncHandlers[opId] = cb;
    }

    function handleAsyncMsgFromRust(opId, buf) {
        if (buf) {
            // This is the overflow_response case of golem::Isolate::poll().
            asyncHandlers[opId](buf);
        } else {
            console.log('open id', opId);

            while (true) {
                const opIdBuf = shift();
                if (opIdBuf == null) {
                    break;
                }
                assert(asyncHandlers[opIdBuf[0]] != null, "Expected async handlers to be non null");
                asyncHandlers[opIdBuf[0]](opIdBuf[1]);
            }
        }
    }

    function dispatch(opId, control, zeroCopy = null) {
        return Golem.core.send(opId, control, zeroCopy);
    }

    const golemCore = {
        setAsyncHandler,
        dispatch,
        sharedQueue: {
            MAX_RECORDS,
            head,
            numRecords,
            size,
            push,
            reset,
            shift,
        },
        ops,
    };

    assert(window[GLOBAL_NAMESPACE] != null, "Expected global ns to be non null");
    assert(window[GLOBAL_NAMESPACE][CORE_NAMESPACE] != null, "Expected global core ns to be non null");
    Object.assign(core, golemCore);
})(this);
