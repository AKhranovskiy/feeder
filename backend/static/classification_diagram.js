const bar_height = 100;
const subbar_width = 10;
const bar_width = subbar_width * 3;
const bar_space = 3;


function draw_classification_diagram(canvas, data) {
    canvas.width = bar_width * data.length + bar_space * data.length - bar_space
    canvas.height = 15 + bar_height + 10;

    let ctx = canvas.getContext("2d");

    data.forEach((p, index) => {

        let max_index = p.indexOf(Math.max(...p));
        let maxed = p.map(function (_, index) { if (index == max_index) { return 10 } else { return 0 }; });

        draw_triple_bar(ctx, index * (bar_width + bar_space), bar_height, ...p.map(x => x * bar_height));
        draw_max_bar(ctx, index * (bar_width + bar_space), bar_height + 5, ...maxed)

        ctx.fillStyle = "red";
        ctx.fillText("A", index * (bar_width + bar_space), 15 + bar_height + 10);
        ctx.fillStyle = "green";
        ctx.fillText("M", index * (bar_width + bar_space) + subbar_width, 15 + bar_height + 10);
        ctx.fillStyle = "blue";
        ctx.fillText("T", index * (bar_width + bar_space) + 2 * subbar_width, 15 + bar_height + 10);
    });
}

function draw_max_bar(ctx, x, y, ads, music, talk) {
    ctx.fillStyle = 'red';
    ctx.fillRect(x, y, bar_width, ads);

    ctx.fillStyle = 'green';
    ctx.fillRect(x, y, bar_width, music);

    ctx.fillStyle = 'blue';
    ctx.fillRect(x, y, bar_width, talk);
}

function draw_triple_bar(ctx, x, y, ads, music, talk) {
    ctx.fillStyle = 'red';
    ctx.fillRect(x, y - ads, subbar_width, ads);

    ctx.fillStyle = 'green';
    ctx.fillRect(x + subbar_width, y - music, subbar_width, music);

    ctx.fillStyle = 'blue';
    ctx.fillRect(x + 2 * subbar_width, y - talk, subbar_width, talk);
}

function draw_stacked_bar(ctx, x, ads, music, talk) {
    ctx.fillStyle = 'red';
    ctx.fillRect(x, 0, bar_width, ads);

    ctx.fillStyle = 'green';
    ctx.fillRect(x, 0 + ads, bar_width, music);

    ctx.fillStyle = 'blue';
    ctx.fillRect(x, 0 + ads + music, bar_width, talk);
}
