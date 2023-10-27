function start() {
    const audioElement = document.querySelector('audio');

    const audioCtx = new window.AudioContext();

    const track = audioCtx.createMediaElementSource(audioElement);
    track.connect(audioCtx.destination);

    const playButton = document.querySelector("button");

    playButton.addEventListener(
        "click",
        () => {
            // Check if context is in suspended state (autoplay policy)
            if (audioCtx.state === "suspended") {
                audioCtx.resume();
            }

            // Play or pause track depending on state
            if (playButton.dataset.playing === "false") {
                audioElement.play();
                playButton.dataset.playing = "true";
            } else if (playButton.dataset.playing === "true") {
                audioElement.pause();
                playButton.dataset.playing = "false";
            }
        },
        false,
    );

    const positionSpan = document.querySelector("#position");
    const durationSpan = document.querySelector("#duration");

    durationSpan.textContent = formatTime(audioElement.duration);

    audioElement.addEventListener(
        "durationchange",
        () => {
            durationSpan.textContent = formatTime(audioElement.duration);
        }
    )
    audioElement.addEventListener(
        "ended",
        () => {
            playButton.dataset.playing = "false";
        },
        false,
    );
    audioElement.addEventListener(
        "timeupdate",
        () => {
            positionSpan.textContent = formatTime(audioElement.currentTime);
            // positionSpan.textContent = audioElement.currentTime;
        },
        false,
    );

    document.addEventListener(
        "keydown",
        (ev) => {
            if (ev.key === " ") {
                audioElement.paused ? audioElement.play() : audioElement.pause();
            }
        },
        false
    );

    const canvas = document.querySelector("canvas");
    canvas.width = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;
    const WIDTH = canvas.width;
    const HEIGHT = canvas.height;
    const MARGIN = 5;

    canvas.addEventListener(
        "click",
        (ev) => {
            if (ev.button == 0) {
                let width = WIDTH - 2 * MARGIN;
                let x = Math.min(Math.max(0, ev.clientX - MARGIN), width);
                audioElement.currentTime = x * audioElement.duration / width;
            }
        },
        false
    );

    const canvasCtx = canvas.getContext("2d");

    let offscreenBack = new OffscreenCanvas(WIDTH, HEIGHT);
    let offscreenBackCtx = offscreenBack.getContext("2d");

    offscreenBackCtx.clearRect(0, 0, WIDTH, HEIGHT);

    offscreenBackCtx.fillStyle = "rgb(250, 250, 250)";
    offscreenBackCtx.fillRect(0, 0, WIDTH, HEIGHT);

    offscreenBackCtx.fillStyle = "rgb(150, 150, 180)";
    offscreenBackCtx.lineWidth = 1;

    let offscreenFront = new OffscreenCanvas(WIDTH, HEIGHT);
    let offscreenFrontCtx = offscreenFront.getContext("2d");

    offscreenFrontCtx.clearRect(0, 0, WIDTH, HEIGHT);

    offscreenFrontCtx.fillStyle = "rgb(250, 250, 250)";
    offscreenFrontCtx.fillRect(0, 0, WIDTH, HEIGHT);

    offscreenFrontCtx.fillStyle = "rgb(0, 0, 180)";
    offscreenFrontCtx.lineWidth = 1;

    let offscreenCursor = new OffscreenCanvas(WIDTH, HEIGHT);
    let offscreenCursorCtx = offscreenCursor.getContext("2d");
    offscreenCursorCtx.globalAlpha = 0.8;
    offscreenCursorCtx.strokeStyle = "rgb(0, 200, 0)";
    offscreenCursorCtx.lineWidth = 2;

    canvas.addEventListener("mouseleave", () => {
        offscreenCursorCtx.clearRect(0, 0, WIDTH, HEIGHT);
    });

    canvas.addEventListener("mousemove", (ev) => {
        const x = Math.max(MARGIN, Math.min(WIDTH - MARGIN, ev.clientX));

        offscreenCursorCtx.clearRect(0, 0, WIDTH, HEIGHT);
        offscreenCursorCtx.beginPath(); // Must be called before drawning otherwise clearing does not work.

        offscreenCursorCtx.save();

        offscreenCursorCtx.strokeStyle = "rgb(0, 100, 0)";
        offscreenCursorCtx.lineWidth = 2;

        offscreenCursorCtx.moveTo(x - 1, 15);
        offscreenCursorCtx.lineTo(x + 3, 15);

        offscreenCursorCtx.moveTo(x + 1, 15);
        offscreenCursorCtx.lineTo(x + 1, 135);

        offscreenCursorCtx.moveTo(x - 1, 135);
        offscreenCursorCtx.lineTo(x + 3, 135);
        offscreenCursorCtx.stroke();

        offscreenCursorCtx.restore();

        offscreenCursorCtx.save();

        offscreenCursorCtx.fillStyle = "rgb(0, 100, 0)";
        offscreenCursorCtx.lineWidth = 1;
        offscreenCursorCtx.font = "normal 16px serif";

        let width = WIDTH - 2 * MARGIN;
        let pos = Math.min(Math.max(0, ev.clientX - MARGIN), width);
        let cursorTime = pos * audioElement.duration / width;

        const cursorSecsText = cursorTime < 10 ? "0" + cursorTime.toFixed(3) : cursorTime.toFixed(3);
        offscreenCursorCtx.fillText(cursorSecsText, x + 4, 15);

        const cursorTimeText = formatTime(cursorTime);
        const cursorTimeMetrix = offscreenCursorCtx.measureText(cursorTimeText);
        offscreenCursorCtx.fillText(cursorTimeText, x - cursorTimeMetrix.width - 4, 135 + cursorTimeMetrix.actualBoundingBoxAscent);

        offscreenCursorCtx.restore();
    })

    function draw() {
        canvasCtx.clearRect(0, 0, WIDTH, HEIGHT);

        canvasCtx.drawImage(offscreenBack, 0, 0, WIDTH, HEIGHT);

        const width = MARGIN + Math.round((WIDTH - 2 * MARGIN) * audioElement.currentTime / audioElement.duration);
        if (width > 0) {
            canvasCtx.drawImage(offscreenFront, 0, 0, width, HEIGHT, 0, 0, width, HEIGHT);
        }

        canvasCtx.drawImage(offscreenCursor, 0, 0, WIDTH, HEIGHT);
        requestAnimationFrame(draw);
    }

    function drawBuffer(buffer) {
        const width = WIDTH - 2 * MARGIN;
        const data = buffer.getChannelData(0);
        const step = Math.ceil(data.length / width);
        const amp = HEIGHT / 2;

        for (var i = 0; i < width; i++) {
            var min = 1.0;
            var max = -1.0;

            for (var j = 0; j < step; j++) {
                var datum = data[(i * step) + j];
                min = Math.min(datum, min);
                max = Math.max(datum, max);
            }

            offscreenBackCtx.fillRect(MARGIN + i, (1 + min) * amp, 1, Math.max(1, (max - min) * amp));
            offscreenFrontCtx.fillRect(MARGIN + i, (1 + min) * amp, 1, Math.max(1, (max - min) * amp));
        }

        draw();
    }

    var audioRequest = new XMLHttpRequest();
    audioRequest.open("GET", audioElement.src, true);
    audioRequest.responseType = "arraybuffer";

    audioRequest.onload = function () {
        audioCtx.decodeAudioData(audioRequest.response, drawBuffer);
    }

    audioRequest.send();

}

// Format time as mm:ss.fff
function formatTime(seconds) {
    // const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60

    const mm = m < 10 ? "0" + m : m;
    const ss = s < 10 ? "0" + s.toFixed(3) : s.toFixed(3);
    return "" + mm + ":" + ss;
}
