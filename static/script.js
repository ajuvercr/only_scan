
async function upload_file(file) {
    var xmlHttpRequest = new XMLHttpRequest();

    var fileName = file.name;
    var mimeType = "plain";

    xmlHttpRequest.open('POST', '/upload', true);
    xmlHttpRequest.setRequestHeader('Content-Type', mimeType);
    xmlHttpRequest.setRequestHeader('Content-Disposition', 'attachment; filename="' + fileName + '"');
    xmlHttpRequest.send(file);

    xmlHttpRequest.onload = function () {
        if (xmlHttpRequest.readyState === xmlHttpRequest.DONE) {
            if (xmlHttpRequest.status === 200) {
                console.log(xmlHttpRequest.response);
                console.log(xmlHttpRequest.responseText);

                elements.textbox.innerHTML += xmlHttpRequest.responseText;
            }
        }
    };
}

const elements = {};

["textbox", "myFileInput", "picPreview", "ref_img"].forEach(x => elements[x] = document.getElementById(x));

function setPicToManipulate() {
    var file = elements.myFileInput.files[0];
    elements.picPreview.src = URL.createObjectURL(file)

    lc_mouseDrag('#inner');
}

function sendPic() {
    var file = elements.myFileInput.files[0];
    const s = {};

    for (let field in file) {
        console.log(field);
        s[field] = file[field];
    }
    upload_file(file).then(x => elements.textbox.innerHTML += x);

    // Send file here either by adding it to a `FormData` object
    // and sending that via XHR, or by simply passing the file into
    // the `send` method of an XHR instance.
}

const image = new Image(); // Using optional size for image
image.onload = drawImageActualSize; // Draw when image has loaded

let drag = undefined;
// Load an image of intrinsic size 300x227 in CSS pixels
image.src = 'test.jpg';

function drawImageActualSize() {
    // Use the intrinsic size of image in CSS pixels for the canvas element
    const w = this.naturalWidth;
    const h = this.naturalHeight;

    const aspect = w / h;


    drag = dragger((ctx, width, height, zoom, offsetx, offsety) => {
        if (ctx instanceof CanvasRenderingContext2D) {
            console.log(width, height, zoom, offsetx, offsety)
            if (width < height * aspect) {
                ctx.drawImage(this, 0, 0, width, width / aspect)
            } else {
                ctx.drawImage(this, 0, 0, height * aspect, height)
            }
        }
    });

    drag.start();
}

function capture_img() {
    console.log("Capturing image")
    const canvas = drag.getCanvas();
    var dataURL = canvas.toDataURL();

    elements.ref_img.src = dataURL;
}


elements.myFileInput.addEventListener('change', setPicToManipulate, false);





// elements.ref_img.addEventListener('load', e => {
//     drag.force_draw()
//     // ctx.drawImage(image, 33, 71, 104, 124, 21, 20, 87, 104);
// });
