# Drawings from photos and scans

When you only have a paper drawing, take a photo of it with a phone or scan it, and balloon the
picture. DimoMark makes the picture a straight sheet at its paper size; you place the balloons and
type the values, as for a PDF without text.

## Taking a good photo

A few seconds of care at the camera save time in **Adjust image** and give sharper balloons:

- **Flat sheet:** lay the drawing on a table and smooth out folds. Weigh down curled corners. A
  bent sheet cannot be made straight afterwards.
- **Even light:** daylight or a ceiling light, no flash. Avoid the shadow of the phone and of your
  hand, and turn the sheet so that no light reflects on it.
- **Whole sheet in the picture:** all four corners of the sheet with a little table around them.
  DimoMark finds the sheet edge for the corner correction only when it can see it.
- **From straight above:** hold the phone parallel to the sheet, over its middle. A small slant is
  corrected with the corners, a strong one leaves small text blurred at the far end.
- **Sharp:** tap the sheet on the screen to focus, hold still, and zoom in afterwards to check
  that the smallest tolerance can be read.
- **No HEIC:** iPhones save photos as HEIC by default, which DimoMark cannot open. Choose
  Settings > Camera > Formats > Most Compatible, or send the photo to yourself by mail or
  messenger, which usually turns it into a JPEG.
- **Scans:** 300 dpi in grey or black and white is enough. Save as PNG or TIFF; a TIFF with
  several pages gives one sheet per page.

## Opening a photo or scan

A photo or scan of a paper drawing opens like a PDF: pick the JPEG, PNG or TIFF file in
**New project**. A TIFF with several pages gives one sheet per page. DimoMark turns a photo the way
the phone recorded it and stores the image unchanged in the project.

- **Size of the sheet:** a scan that states its resolution (100 to 1200 dpi) gets its real size.
  A photo does not know its size, so its long side is set to 297 mm, the length of an A4 sheet.
  Large images are kept at no more than 300 dpi at that size.
- **No text in the image:** a box drawn around a dimension finds no text, so the balloon is placed
  and you type the value, as for a PDF without text.
- **HEIC photos** cannot be opened, see the tips above.
- The characteristic list, the reports and the ballooned PDF work as for a PDF. The ballooned PDF
  shows the adjusted sheet at its paper size ([Exports](exports.md)).

## Adjust image

**Drawing > Adjust image** (also the first step of **Set up this drawing** after opening a photo) makes a
photo a straight sheet. The original image stays in the project unchanged; the settings can be
changed again at any time.

1. **Rotate:** turn in 90 degree steps, then straighten with the slider (up to 10 degrees either
   way) or type an angle (up to 45 degrees; positive turns clockwise).
2. **Corners:** for a photo taken at a slant, switch on "Correct the corners". DimoMark looks for
   the edge of the sheet and puts the four handles on its corners; if it does not find it, the
   handles start at the image corners. Drag them onto the corners of the sheet. The corners
   straighten the sheet themselves, so the angle is set back to 0.
3. **Trim:** drag the corners of the rectangle, or the rectangle itself, to keep only the sheet.
4. **Paper:** pick the paper the photo shows (A4 to A0, Letter, Tabloid or a custom size in mm)
   and portrait or landscape. The trimmed image is put on that sheet, so measured sizes fit.
   "From the photo" keeps the size of the photo.
5. **Clean up** (optional) shows the image in grey with a white background and stronger lines.

**Apply** is one step that **Undo** takes back. Balloons already placed keep their place on the
drawing. If the new sheet would leave balloons outside, DimoMark asks first; "Apply anyway" puts
them on the edge of the sheet. **Reset** goes back to the image as it was opened.
