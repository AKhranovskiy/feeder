let eventList = document.getElementById("events")
let eventItemTemplate = document.getElementById("event")

function addEvent(event) {
    var node = eventItemTemplate.content.cloneNode(true);
    node.querySelector("li").textContent = event;
    eventList.prepend(node);
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

function load_segments(uri) {
    fetch(uri)
        .then(res => res.json())
        .then((segments) => {
            console.log('Segments: ', segments);
            showSegments(segments)
        })
        .catch(err => console.error(err));
}

function showSegments(segments) {
    const segmentsTableBody = document.getElementById("segments-table-body");
    const segmentsTableRow = document.getElementById("segments-table-row");

    segmentsTableBody.querySelectorAll("tr").forEach(function (e) { e.remove() })

    for (const segment of segments) {
        var row = segmentsTableRow.content.cloneNode(true);
        row.querySelector("tr").classList.add(segment.kind);
        row.querySelector(".datetime").textContent = segment.date_time;
        row.querySelector(".content-kind a").onclick = function (ev) { load_segments(`/api/v1/segments/kind/${segment.kind}?skip=0&limit=10`) };
        row.querySelector(".content-kind a").href = "#"
        row.querySelector(".content-kind a").textContent = segment.kind;
        row.querySelector(".artist").textContent = segment.artist;
        row.querySelector(".title").textContent = segment.title;
        row.querySelector(".audio a").href = `/api/v1/segment/${segment.id}/audio`;
        row.querySelector(".matches").textContent = segment.number_of_matches;
        segmentsTableBody.appendChild(row)
    }
}

function init() {
    subscribe("/api/v1/events")
    load_segments("/api/v1/segments/json")
}

init()
