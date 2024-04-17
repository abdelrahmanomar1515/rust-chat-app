const socket = new WebSocket("ws://127.0.0.1:8080/ws");

function scrollToBottom() {
    const messageList = jQuery("#message-list");
    const newMessage = messageList.children("li:last-child");

    const clientHeight = messageList.prop("clientHeight");
    const scrollTop = messageList.prop("scrollTop");
    const scrollHeight = messageList.prop("scrollHeight");
    const newMessageHeight = newMessage.innerHeight();
    const lastMessageHeight = newMessage.prev().innerHeight();

    if (
        clientHeight + scrollTop + newMessageHeight + lastMessageHeight >=
        scrollHeight
    ) {
        messageList.scrollTop(scrollHeight);
    }
}

socket.addEventListener("open", function() {
    const params = jQuery.deparam(window.location.search);

    socket.send(JSON.stringify(params));
});

socket.addEventListener("close", function() {
    console.log("disconnected");
});

socket.addEventListener("message", function(msg) {
    const message = JSON.parse(msg.data);
    if (message.type === "updateUserList") {
        let ol = jQuery("<ol></ol>");
        for (let user of msg) {
            ol.append(jQuery("<li></li>").text(user));
        }
        jQuery("#users").html(ol);
    }
});

socket.addEventListener("message", function(msg) {
    const message = JSON.parse(msg.data);
    if (message.text) {
        const template = jQuery("#message-template").html();
        const html = Mustache.render(template, {
            text: message.text,
            from: message.from,
            createdAt: message.timeStamp,
        });
        jQuery("#message-list").append(html);
        scrollToBottom();
    }
});

jQuery("#message-form").on("submit", (e) => {
    e.preventDefault();
    const messageTextbox = jQuery("#message-form-message");
    socket.send(
        JSON.stringify({ type: "receivedMessage", content: messageTextbox.val() }),
    );
    messageTextbox.val("");
});
