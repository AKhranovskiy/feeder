class Category {
    static Advertisement = new Category("Advertisement")
    static Music = new Category("Music")
    static Talk = new Category("Talk")
    static Unknown = new Category("Unknown")

    constructor(name) {
        this.name = name
    }

    get color() {
        if (this.name == Category.Advertisement.name) return 'red';
        if (this.name == Category.Music.name) return 'green';
        if (this.name == Category.Talk.name) return 'blue';
        if (this.name == Category.Unknown.name) return 'grey';
		return 'black';
    }
}

const VisualisationSettings = Object.freeze({
    bar: {
        width: 10,
        padding: {
            vertical: 2,
            horizontal: 2
        },
        spacing: 1,
        emptyColor: 'snow'
    },
    activeBar: {
        outline: {
            color: 'rgba(255,255,255,0.8)',
            width: 1
        },
        color: 'rgba(255,255,255,0.5)'
    }
});

class Stream {
    #player = null;
	#audio = null;

    constructor(domElement) {
        this.#player = new Player(domElement.querySelector('.player'));
		this.#audio = new Audio();
    }

	addSegment(segment) {
		const player = this.#player;
		for (const [kind, conf] of segment.classification) {
			const category = new Category(kind);

			const bar = new Bar(
				// TODO - give unique id.
				segment.id,
				segment.url,
				segment.artist,
				segment.title,
				new Category(kind),
				conf,
			);

			delay(200).then(function() {
				player.push(bar)
			});
		}
		new Audio(segment.url).addEventListener("canplaythrough", (event) => {
			this.#audio.pause();
			event.target.play();
			this.#audio = event.target;
		});
	}
}

const delay = (milliseconds) => new Promise(resolve => {
    setTimeout(resolve, milliseconds);
});

class Bar {
    constructor(id, source, artist, title, category, confidence) {
        this.id = id;
		this.source = source;
		this.artist = artist;
		this.title = title;
        this.category = category;
		this.confidence = confidence;
    }
}

class Player {
    #visualisation = null;
    #bars = [];

    constructor(domElement) {
        this.#visualisation = new Visualisation(domElement.querySelector('.visualisation'))
    }

    push(bar) {
        this.#bars.unshift(bar)
        this.#visualisation.update({
            bars: this.#bars
        });
    }
}

class Visualisation {
    #barsCanvas = null;
    #activeBarCanvas = null;
    #playPositionCanvas = null;
    #activeZoneDiv = null;
    #tooltipDiv = null;
    #bars = [];
    #width = 0;
    #height = 0;
    #activeBar = null;
    #lastMousePos = null;

    constructor(domElement) {
        this.#width = domElement.clientWidth;
        this.#height = domElement.clientHeight;

        this.#barsCanvas = this.#get_canvas_context(domElement, 'canvas.bars');
        this.#activeBarCanvas = this.#get_canvas_context(domElement, 'canvas.active-bar');
        this.#playPositionCanvas = this.#get_canvas_context(domElement, 'canvas.play-position');
        this.#activeZoneDiv = domElement.querySelector('div.active-zone');
        this.#tooltipDiv = domElement.querySelector('div.tooltip');

        const self = this;
        this.#activeZoneDiv.addEventListener('mousemove', function (ev) {
            self.onmousemove(ev.clientX, ev.clientY);
            ev.preventDefault();
        });
        this.#activeZoneDiv.addEventListener('mouseout', function (ev) {
            self.onmouseout(ev.clientX, ev.clientY);
            ev.preventDefault();
        });
    }

    onmousemove(x, y) {
        this.#lastMousePos = {
            x: x,
            y: y
        };
        this.#activeBar = this.#bars.find(b => b.rect.x <= x && x <= (b.rect.x + b.rect.width)) || null;
        this.#drawActiveBar();
        this.#showTooltip();
    }
    onmouseout(x, y) {
        this.#lastMousePos = null;
        this.#activeBar = null;
        this.#drawActiveBar();
        this.#showTooltip();
    }

    #get_canvas_context(domElement, selector) {
        const canvas = domElement.querySelector(selector);
        canvas.width = canvas.clientWidth;
        canvas.height = canvas.clientHeight;
        return canvas.getContext('2d');
    }

    update(data) {

        this.#bars = this.#calculateRects(data.bars || []);
        this.#drawBars();

        if (this.#lastMousePos != null) {
            const {
                x,
                y
            } = this.#lastMousePos;
            this.onmousemove(x, y);
        } else if (this.#activeBar != null) {
            const id = this.#activeBar.bar.id;
            this.#activeBar = this.#bars.find(b => b.bar.id == id) || null;
            this.#drawActiveBar();
        }
    }

    #calculateRects(bars) {
        const width = this.#width;
        const height = this.#height;

        const vertPadding = VisualisationSettings.bar.padding.vertical;
        const horPadding = VisualisationSettings.bar.padding.horizontal;
        const spacing = VisualisationSettings.bar.spacing;

        const barWidth = VisualisationSettings.bar.width;
        const barHeight = height - vertPadding * 2;

        return bars.map((bar, idx) => Object({
            bar: bar,
            color: bar.category.color,
            rect: {
                x: width - (horPadding + idx * (spacing + barWidth) + barWidth),
                y: vertPadding,
                width: barWidth,
                height: barHeight,
            }
        }));
    }

    #drawBars() {
        const ctx = this.#barsCanvas;
        ctx.save();

        ctx.fillStyle = VisualisationSettings.bar.emptyColor;
        ctx.fillRect(0, 0, this.#width, this.#height);

        for (const bar of this.#bars) {
            ctx.fillStyle = bar.color;
            ctx.fillRect(bar.rect.x, bar.rect.y, bar.rect.width, bar.rect.height);
        }
        ctx.restore();
    }

    #drawActiveBar() {
        const ctx = this.#activeBarCanvas;
        ctx.save();

        ctx.clearRect(0, 0, this.#width, this.#height);

        const bar = this.#activeBar;

        if (bar != null) {
            ctx.strokeStyle = VisualisationSettings.activeBar.outline.color;
            ctx.lineWidth = VisualisationSettings.activeBar.outline.width;
            ctx.fillStyle = VisualisationSettings.activeBar.color;

            ctx.beginPath();
            ctx.rect(bar.rect.x, bar.rect.y, bar.rect.width, bar.rect.height);
            ctx.closePath();

            ctx.fill();
            ctx.stroke();
        }

        ctx.restore();
    }

    #showTooltip() {
        const tooltip = this.#tooltipDiv;
        const bar = this.#activeBar;

        if (bar != null) {
            const offset = this.#bars.indexOf(bar);
            tooltip.style.left = `${15 + bar.rect.x}px`;
            tooltip.style.visibility = 'visible';
            tooltip.style.opacity = '1';
            tooltip.innerText = this.#getTooltipText(bar.bar);
        } else {
            tooltip.style.visibility = 'hidden';
            tooltip.style.opacity = '0';
            tooltip.innerText = ``;
        }
    }

	#getTooltipText(bar) {
		return `ID: ${bar.id}\n` +
			`Source: ${bar.source}\n` +
			`Artist: ${bar.artist}\n` +
			`Title: ${bar.title}\n` +
			`Category: ${bar.category.name}\n` +
			`Confidence: ${bar.confidence}`;
	}
}


window.onload = function () {
	loadStreams()
		.then(processStreams)
		.then(subscribeForUpdates)
		.catch(e => console.error(e));
}

async function loadStreams() {
    const response = await fetch(
		'/api/v1/streams',
		{
			method: 'GET',
			headers: {'Accept': 'application/json'},
		}
	);
    return response.json();
}

async function processStreams(streams) {
	window.streams = new Map();

	for (const stream of streams) {
		const template = document.querySelector('template#stream');
		let node = template.content.cloneNode(true);
		let article = node.querySelector('article');
		article.id = stream.id;

		article.querySelector('header > h1 > span').textContent = stream.name;
		article.querySelector('header > h1 > a').href = stream.url;
		document.querySelector('main').appendChild(node);

		window.streams.set(stream.id, new Stream(article));
	}
}

function subscribeForUpdates() {
	const sse = new EventSource('/api/v1/playbacks/updates');
	sse.onerror = (e) => {
		console.error("Update subcription failed");
		sse.close()
	}

	sse.addEventListener('add', (ev) => {
		let playback =JSON.parse(ev.data); 
		console.debug("ADD", ev.lastEventId, playback);

		window.streams.get(playback.stream_id).addSegment(playback);
	});
	sse.addEventListener('delete', (ev) => {
		console.debug("DELETE", ev.lastEventId);
	});
	sse.addEventListener('error', (ev) => {
		console.error("ERROR", ev.lastEventId);
	});
}
