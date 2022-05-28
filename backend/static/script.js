let eventList = document.getElementById("events")
let eventItemTemplate = document.getElementById("event")

function addEvent(event) {
    var node = eventItemTemplate.content.cloneNode(true);
    node.querySelector("li").textContent = event;
    eventList.appendChild(node);
}

// Subscribe to the event source at `uri` with exponential backoff reconnect.
function subscribe(uri) {
    var retryTime = 1;

    function connect(uri) {
        const events = new EventSource(uri);

        events.addEventListener("new-segment", (ev) => {
            const event = JSON.parse(ev.data)["NewSegment"];
            console.log("decoded data", JSON.stringify(event));

            addEvent(`${(new Date()).toISOString()} new segment: ${event.id} ${event.kind} ${event.artist} ${event.title}`)
        });

        events.addEventListener("match", (ev) => {
            const event = JSON.parse(ev.data)["Match"];
            console.log("decoded data", JSON.stringify(event));

            addEvent(`${(new Date()).toISOString()} matched segment: ${(event.score * 100 / 255).toFixed()} ${event.id} ${event.kind} ${event.artist} ${event.title}`)
        });

        events.addEventListener("open", () => {
            console.log(`connected to event stream at ${uri}`);
            retryTime = 1;
        });

        events.addEventListener("error", () => {
            events.close();

            let timeout = retryTime;
            retryTime = Math.min(64, retryTime * 2);
            console.log(`connection lost. attempting to reconnect in ${timeout}s`);
            setTimeout(() => connect(uri), (() => timeout * 1000)());
        });
    }

    connect(uri);
}

function init() {
    subscribe("/api/v1/events")
}

init()
