
const dragger = function (element, horizontal, vertical) {
    let start_x = 0;
    let start_y = 0;
    let current_x = 0;
    let current_y = 0;
    let changed = true;

    function flush() {
        if (changed) {
            horizontal(start_x - current_x);
            vertical(start_y - current_y);
        }

        start_x = current_x;
        start_y = current_y;

        requestAnimationFrame(flush);
    }

    // Gets the relevant location from a mouse or single touch event
    function getEventLocation(e) {
        if (e.touches && e.touches.length == 1) {
            return [e.touches[0].clientX, e.touches[0].clientY];
        }
        else if (e.clientX && e.clientY) {
            return [e.clientX, e.clientY];
        }
    }

    let isDragging = false

    function onPointerDown(e) {
        isDragging = true;

        const [x, y] = getEventLocation(e);
        start_x = (start_x - current_x) + x;
        start_y = (start_y - current_y) + y;
    }

    function onPointerUp(e) {
        isDragging = false
    }

    function onPointerMove(e) {
        if (isDragging) {
            const [x, y] = getEventLocation(e);

            current_x = x;
            current_y = y;
            changed = true;
        }
    }

    function handleTouch(e, singleTouchHandler) {
        if (e.touches.length == 1) {
            singleTouchHandler(e)
        }
    }

    element.addEventListener('mousedown', onPointerDown)
    document.addEventListener('touchstart', (e) => handleTouch(e, onPointerDown))
    document.addEventListener('mouseup', onPointerUp)
    document.addEventListener('touchend', (e) => handleTouch(e, onPointerUp))
    document.addEventListener('mousemove', onPointerMove)
    document.addEventListener('touchmove', (e) => handleTouch(e, onPointerMove))

    // Ready, set, go
    flush();

    return this;
};
