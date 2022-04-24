frame_num = 0
fn_format = "Screenshots/%d.png"
while true do
    frame_num = frame_num + 1
    filename = string.format(fn_format, frame_num)
    print(string.format("Saving file to %s", filename))
    client.screenshot(filename)
	emu.frameadvance();
end
