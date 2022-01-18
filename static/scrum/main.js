function build_form(obj, action, method="POST") {
    const form = document.createElement("form");
    form.method = method;
    form.action = action;

    for (const [key, value] of Object.entries(obj)) {
        console.log(key, value);
        const inp = document.createElement("input");
        inp.type = "hidden";
        inp.value = value;
        inp.name = key;
        form.appendChild(inp);
    }

    document.body.appendChild(form);
    return form;
}

function onDragOver(event) {
    event.preventDefault();
}

var dragged;

function onDrop(event) {
    event.preventDefault();

    delete dragged.style.opacity;

    let target = event.target;
    while(!target.classList.contains("story")) {
        target = target.parentElement;
    }

    if(target == dragged) return;
    const new_parent = target.dataset.story;


    const [story, parent] = event
        .dataTransfer
        .getData('text').split();

    console.log("parent", parent, "story", story, "new_parent", new_parent);

    fetch(`/scrum/${story}`, {
       method: "PATCH",
       headers: {  
        'Content-Type': 'application/json'
       },
       body: JSON.stringify({"parent": new_parent})
     })


    const createXhr = new XMLHttpRequest();
    createXhr.open('POST', `/scrum/${new_parent}/sub/${story}`);
    createXhr.send();

    if (parent) {
        const deleteXhr = new XMLHttpRequest();
        deleteXhr.open('DELETE', `/scrum/${parent}/sub/${story}`);
        deleteXhr.send();
    }
}

function onDragStart(event) {
    let target = event.target;
    while(!target.classList.contains("story")) {
        target = target.parentElement;
    }
    dragged = target;

    target.style.opacity = 0;
    event
        .dataTransfer
        .setData('text/plain', target.dataset.story + " " + target.dataset.parent);
}
