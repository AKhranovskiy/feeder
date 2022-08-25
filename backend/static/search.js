
function* filter(iterator, predicat) {
    var next = iterator.next();
    while (!next.done) {
        const value = next.value;
        if (predicat(value)) {
            yield value;
        }
        next = iterator.next();
    }
}

const value_not_empty = ([key, value]) => value !== '';

function search(form) {
    const url = new URL(form.action);
    const entries = new FormData(form).entries();
    const queryString = new URLSearchParams([
        ...Array.from(url.searchParams.entries()),
        ...filter(entries, value_not_empty),
        ['limit', 50]
    ]).toString();


    const new_url = new URL(`${url.origin}${url.pathname}?${queryString}`);

    makeRequest('GET', new_url, null)
        .then((response) => console.log("Response: " + JSON.stringify(response)))
        .catch((err) => console.error(err));
}

async function makeRequest(method, url, body) {
    // Default options are marked with *
    const response = await fetch(url, {
        method: method, // *GET, POST, PUT, DELETE, etc.
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: body
    });
    return response.json(); // parses JSON response into native JavaScript objects
}

function reset_form(form) {
    for (const element of [...form.elements].filter(e => e.nodeName == 'INPUT')) {
        switch (element.type) {
            case 'text':
                element.value = '';
                break;
            case 'checkbox':
                element.checked = false;
                break;

            default:
                console.log('Unknown input element, type=' + element.type + ', id=' + element.id);
                break;
        }
    }
}


function first_page(limit) {
    document.location = update_query_params(
        new URL(document.location),
        [['skip', 0], ['limit', limit]]
    )
}

function prev_page(skip, limit) {
    document.location = update_query_params(
        new URL(document.location),
        [['skip', Math.max(0, skip - limit)], ['limit', limit]]
    )
}

function next_page(skip, limit) {
    document.location = update_query_params(
        new URL(document.location),
        [['skip', skip + limit], ['limit', limit]]
    )
}

function last_page(limit, total) {
    document.location = update_query_params(
        new URL(document.location),
        [['skip', Math.max(0, total - limit)], ['limit', limit]]
    )
}

function update_query_params(url, new_params) {
    var params = url.searchParams.entries();
    var params = filter(params, value_not_empty);
    var params = filter(params, not(keys(new_params.map(([k, v]) => k))));
    var params = Array.from(params);
    const queryString = new URLSearchParams([
        ...params,
        ...new_params
    ]).toString();

    return new URL(`${url.origin}${url.pathname}?${queryString}`);
}
// Returns a function that returns true if `keys` includes `key` from `[key, value]` argument.
const keys = (keys) => ([key, value]) => keys.includes(key);
// Return a function that negates the result of `expr`.
const not = (expr) => (args) => !expr(args);

function select_row(row) {
    if (row.dataset.selected == undefined ||
        row.dataset.selected == "false") {
        row.dataset.selected = "true";
    } else {
        row.dataset.selected = "false";
    }
}

function select_all_rows(table) {
    while (table != null && table.nodeName != "TABLE") {
        table = table.parentNode;
    }
    let rows = Array.from(table.tBodies[0].rows);
    let selected = (rows.map((r) => r.dataset.selected == "true").includes(false)) ? "true" : "false";
    console.debug(selected);

    for (const row of rows) {
        row.dataset.selected = selected
    }
}

async function delete_item(e, id) {
    e.preventDefault();
    e.stopPropagation();

    await delete_item_impl(id);
    window.location.reload();

    return false;
}

async function delete_many_items(event, items) {
    event.preventDefault();
    event.stopPropagation();

    await Promise.all(items.map((item) => delete_item_impl(item)));

    window.location.reload();
    return false;
}

async function delete_item_impl(id) {
    await fetch('/api/v1/segment/' + id, { method: 'DELETE', })
        .then((response) => {
            if (response.ok) {
                console.info("Item " + id + " deleted successfully.");
            } else {
                console.error(response.status, "Item " + id + " is not deleted");
            }
        }).catch((error) => {
            console.error(error);
        });
}

function copy_media_url(path) {
    navigator.clipboard.writeText(new URL(path, document.location.origin).toString());
}
