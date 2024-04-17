const socket = new WebSocket("ws://127.0.0.1:8080/ws");

socket.addEventListener("open", function(event) {
    socket.send("Hello Server!");
});

socket.addEventListener("message", function(event) {
    console.log("Message from server ", event.data);
});

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
    // socket.send("join", params, function(err) {
    //     if (err) {
    //         alert(err);
    //         window.location.href = "/";
    //     } else {
    //         console.log("no error");
    //     }
    // });
    socket.send("Join please");
});

socket.addEventListener("close", function() {
    console.log("disconnected");
});

socket.addEventListener("message", function(users) {
    if (users.data.startsWith("updateUserList")) {
        let ol = jQuery("<ol></ol>");
        for (let user of users) {
            ol.append(jQuery("<li></li>").text(user));
        }
        jQuery("#users").html(ol);
    }
});

socket.addEventListener("message", function(msg) {
    if (msg.data.startsWith("newMsg,")) {
        const formattedTime = moment().format("H:mm");
        const template = jQuery("#message-template").html();
        const html = Mustache.render(template, {
            text: msg.data,
            from: "test",
            createdAt: formattedTime,
        });
        jQuery("#message-list").append(html);
        scrollToBottom();
    }
});

jQuery("#message-form").on("submit", (e) => {
    e.preventDefault();
    const messageTextbox = jQuery("#message-form-message");
    socket.send(["msg", messageTextbox.val()]);
    messageTextbox.val("");
});