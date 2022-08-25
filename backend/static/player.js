var player = null;
var intervalId = null;
var percent = 0;

function initialize_player() {
    if (typeof Howl === 'undefined') {
        console.error("Missing Howl library.")
        return false;
    }
}

function player_toggle(elem) {
    if (player != null) {
        clearInterval(intervalId);
        percent = 0;
        player.stop();
        player = null;
    }

    if (elem.dataset.playState == 'stop') {
        for (const btn of Array.from(
            document.querySelectorAll('.playbutton[data-play-state="play"]').values()
        )) {
            btn.dataset.playState = "stop"
            btn.style = ''
        }

        player = new Howl({
            src: elem.dataset.mediaUrl,
            html5: true,
            autoplay: true,
            rate: 2.0,
            format: ['aac'],
            onload: () => {
                elem.dataset.playState = 'play'
            },
            onstop: () => {
                clearInterval(intervalId);
                percent = 0;
                elem.dataset.playState = 'stop'
                elem.style = '';
            },
            onend: () => {
                clearInterval(intervalId);
                percent = 0;
                elem.dataset.playState = 'stop'
                elem.style = '';
            }
        });

        player.once('play', (id) => {
            let rate = player.rate();
            let duration = player.duration() * 1000 / rate;
            let interval = duration / 100;

            intervalId = setInterval(() => {
                elem.style = '--percent:' + percent;
                percent += 1;
            }, interval);
        });
    }
}
