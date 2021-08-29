
const dragger = function (draw_f) {

    let canvas = document.getElementById("canvas")
    let ctx = canvas.getContext('2d')

    let width, heigh = 0;

    let cameraOffset = { x: 0, y: 0 }
    let cameraZoom = 1
    let MAX_ZOOM = 5
    let MIN_ZOOM = 0.1
    let SCROLL_SENSITIVITY = 0.0005
    let changed = true;

    function set_sizing() {
        const box = canvas.getBoundingClientRect();
        width = box.width;
        height = box.height;
        canvas.width = width;
        canvas.height = height;

    }

    function do_draw() {
        set_sizing();

        // Translate to the canvas centre before zooming - so you'll always zoom on what you're looking directly at
        ctx.translate(width / 2, height / 2)
        ctx.scale(cameraZoom, cameraZoom)
        ctx.translate(-width / 2 + cameraOffset.x, -height / 2 + cameraOffset.y)
        ctx.clearRect(0, 0, width, height)
        // ctx.fillStyle = "#991111"

        draw_f(ctx, width, height, cameraZoom, width / 2 + cameraOffset.x, height / 2 + cameraOffset.y)
    }

    this.force_draw = do_draw;


    function draw() {
        if (changed) {
            changed = false;
            do_draw();
        }

        requestAnimationFrame(draw)
    }

    this.getCanvas = () => canvas;

    // Gets the relevant location from a mouse or single touch event
    function getEventLocation(e) {
        if (e.touches && e.touches.length == 1) {
            return { x: e.touches[0].clientX, y: e.touches[0].clientY }
        }
        else if (e.clientX && e.clientY) {
            return { x: e.clientX, y: e.clientY }
        }
    }

    function drawRect(x, y, width, height) {
        ctx.fillRect(x, y, width, height)
    }

    function drawText(text, x, y, size, font) {
        ctx.font = `${size}px ${font}`
        ctx.fillText(text, x, y)
    }

    let isDragging = false
    let dragStart = { x: 0, y: 0 }

    function onPointerDown(e) {
        isDragging = true
        dragStart.x = getEventLocation(e).x / cameraZoom - cameraOffset.x
        dragStart.y = getEventLocation(e).y / cameraZoom - cameraOffset.y
    }

    function onPointerUp(e) {
        isDragging = false
        initialPinchDistance = null
        lastZoom = cameraZoom
    }

    function onPointerMove(e) {
        if (isDragging) {
            cameraOffset.x = getEventLocation(e).x / cameraZoom - dragStart.x
            cameraOffset.y = getEventLocation(e).y / cameraZoom - dragStart.y
            changed = true;
        }
    }

    function handleTouch(e, singleTouchHandler) {
        if (e.touches.length == 1) {
            singleTouchHandler(e)
        }
        else if (e.type == "touchmove" && e.touches.length == 2) {
            isDragging = false
            handlePinch(e)
        }
    }

    let initialPinchDistance = null
    let lastZoom = cameraZoom

    function handlePinch(e) {
        e.preventDefault()

        let touch1 = { x: e.touches[0].clientX, y: e.touches[0].clientY }
        let touch2 = { x: e.touches[1].clientX, y: e.touches[1].clientY }

        // This is distance squared, but no need for an expensive sqrt as it's only used in ratio
        let currentDistance = (touch1.x - touch2.x) ** 2 + (touch1.y - touch2.y) ** 2

        if (initialPinchDistance == null) {
            initialPinchDistance = currentDistance
        }
        else {
            adjustZoom(null, currentDistance / initialPinchDistance)
        }
    }

    function adjustZoom(zoomAmount, zoomFactor) {
        if (!isDragging) {
            if (zoomAmount) {
                cameraZoom += zoomAmount
            }
            else if (zoomFactor) {
                console.log(zoomFactor)
                cameraZoom = zoomFactor * lastZoom
            }

            cameraZoom = Math.min(cameraZoom, MAX_ZOOM)
            cameraZoom = Math.max(cameraZoom, MIN_ZOOM)
            changed = true;

            console.log(zoomAmount)
        }
    }

    window.addEventListener('resize', () => {
        changed = true;
    });

    canvas.addEventListener('mousedown', onPointerDown)
    canvas.addEventListener('touchstart', (e) => handleTouch(e, onPointerDown))
    canvas.addEventListener('mouseup', onPointerUp)
    canvas.addEventListener('touchend', (e) => handleTouch(e, onPointerUp))
    canvas.addEventListener('mousemove', onPointerMove)
    canvas.addEventListener('touchmove', (e) => handleTouch(e, onPointerMove))
    canvas.addEventListener('wheel', (e) => adjustZoom(e.deltaY * SCROLL_SENSITIVITY))

    // Ready, set, go
    this.start = () => draw();

    return this;
};
