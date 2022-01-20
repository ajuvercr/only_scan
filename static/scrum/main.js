function onDragOver(event) {
    event.preventDefault();
}

var dragged;

async function onDrop(event) {
 event.stopPropagation() ;  event.preventDefault();

    delete dragged.style.opacity;

    let target = event.target;
    while(!target.classList.contains("story")) {
        target = target.parentElement;
    }

    if(target == dragged) return;
    const new_parent = target.dataset.story;


    const [story, parent] = event
        .dataTransfer
        .getData('text').split(" ");

    console.log("parent", parent, "story", story, "new_parent", new_parent);

    await fetch(`/scrum/${story}`, {
       method: "PATCH",
       headers: {  
        'Content-Type': 'application/json'
       },
       body: JSON.stringify({"parent": new_parent})
     })

    await fetch(`/scrum/${new_parent}/sub/${story}`, {method: "POST"});


    if (parent) {
        await fetch(`/scrum/${parent}/sub/${story}`, {method: "DELETE"});
    }

    location.reload();
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

const editing = new Set();
function handleEdit(event) {
    const story = event.target.parentElement.parentElement;

    const id = story.dataset.story;
    if(editing.has(id)) {
        event.target.innerText = "Edit";
        editing.delete(id);
        story.setAttribute("draggable", "true");

        const fields = story.getElementsByClassName("field");
        const obj = {};

        for(let i = 0; i < fields.length; i++) {
            const field = fields[i];
          if(field.dataset.for === id) {
            field.setAttribute("contentEditable", "false");
              
              const f = field.dataset.get || "innerText";
              if(field.dataset.original !== field[f]) {
                obj[field.dataset.field] = field[f];
              }
          }
        }
        console.log(obj)

        fetch(`/scrum/${id}`, {
           method: "PATCH",
           headers: {  
            'Content-Type': 'application/json'
           },
           body: JSON.stringify(obj)
         }).then(console.log);
    } else {
        event.target.innerText = "Save";
        editing.add(id);

        story.setAttribute("draggable", "false");

        const fields = story.getElementsByClassName("field");

        for(let i = 0; i < fields.length; i++) {
          if(fields[i].dataset.for === id)
            fields[i].setAttribute("contentEditable", "true");
        }
        console.log(fields);
    }
}


function toggleDone(event) {
    const cl = event.target.classList;

    let out = false;
    if(cl.contains("true")) {
        out = false;
        cl.remove("true");
        cl.add("false");
    } else {
        out = true;
        cl.remove("false");
        cl.add("true");
    }
    const obj = {done: out};
    fetch(`/scrum/${event.target.dataset.for}`, {
       method: "PATCH",
       headers: {  
        'Content-Type': 'application/json'
       },
       body: JSON.stringify(obj)
     }).then(console.log);
}







