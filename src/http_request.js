(function(global) {
    const OP_NAME = 'op_http_request';
    async function http_request(args) {

        console.log('calling http_request');
        let body = new Uint8Array();
        const results = await Golem.core.sendAsync(
            OP_NAME,
            args = {},
            body
        );
        console.log(results.body);
    };
    global.http_request =  http_request;
    Golem.core.setAsyncHandler(Golem.core.OPS_CACHE[OP_NAME], Golem.core.asyncMsgFromRust);
})(this);
