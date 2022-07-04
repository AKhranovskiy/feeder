// let eventList = document.getElementById("events")
// let eventItemTemplate = document.getElementById("event")

// function addEvent(event) {
//     var node = eventItemTemplate.content.cloneNode(true);
//     node.querySelector("li").textContent = event;
//     eventList.prepend(node);
// }

// Subscribe to the event source at `uri` with exponential backoff reconnect.
function subscribe(uri, callback) {
    var retryTime = 1;

    function connect(uri) {
        const events = new EventSource(uri);

        events.onmessage = ev => callback(JSON.parse(ev.data));
        // events.addEventListener("new-segment", (ev) => {
        //     const event = JSON.parse(ev.data)["NewSegment"];
        //     console.log("decoded data", JSON.stringify(event));

        //     addEvent(`${(new Date()).toISOString()} new segment: ${event.id} ${event.kind} ${event.artist} ${event.title}`)
        // });

        // events.addEventListener("match", (ev) => {
        //     const event = JSON.parse(ev.data)["Match"];
        //     console.log("decoded data", JSON.stringify(event));

        //     addEvent(`${(new Date()).toISOString()} matched segment: ${(event.score * 100 / 255).toFixed()} ${event.id} ${event.kind} ${event.artist} ${event.title}`)
        // });

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


function load_json(uri, callback) {
    fetch(uri)
        .then(res => res.json())
        .then(callback)
        .catch(err => console.error(err));
}

const segmentsTableBody = document.getElementById("segments-table-body");
const segmentsTableRow = document.getElementById("segments-table-row");

function populate_metadata(uri, skip, limit) {
    load_json(`${uri}?skip=${skip}&limit=${limit}`, function (items) {

        segmentsTableBody.querySelectorAll("tr").forEach(function (e) { e.remove() })

        for (const item of items) {
            var row = segmentsTableRow.content.cloneNode(true);
            fill_metadata_row(row, item)
            segmentsTableBody.append(row)
        }
    })
}

function fill_metadata_row(row, metadata) {
    row.querySelector(".datetime").textContent = new Date(metadata.date_time).toLocaleString();
    row.querySelector(".content-kind").classList.add(metadata.kind);
    // row.querySelector(".content-kind a").href = `/filter/kind/${metadata.kind}`
    row.querySelector(".content-kind a").textContent = metadata.kind;
    row.querySelector(".artist").textContent = metadata.artist;
    row.querySelector(".title").textContent = metadata.title;
    row.querySelector(".audio audio").src = `/api/v1/segment/${metadata.id}/audio`;
    row.querySelector(".view a").href = `/view/${metadata.id}`
}
