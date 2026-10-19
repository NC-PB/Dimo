# Manual test script, photos and scans (release 0.1)

For what `scripts/e2e-smoke.sh` cannot reach with drawings from photos and scans (R.19 to R.22):
a real phone photo with EXIF orientation, a slanted photo with the corner correction, the drag
handles of corners and trim, a multi-page TIFF, a scan with its resolution, HEIC, the printed
size of the ballooned PDF and the user guide page. Smoke step 12 covers the rest: a JPEG turned
a quarter, rotate, fine angle, A3 landscape, clean up, two typed balloons, save, export and a
byte identical re-export. It follows the same idea as [manual-test-m3.md](manual-test-m3.md) and
takes about 40 minutes. Run it before the 0.1 release and after changes to image opening, the
image editor or the image export. Record the result in the table at the end and put the notes in
the pull request or in `docs/plan/STATUS.md`.

## Before you start

1. Run `./scripts/e2e-smoke.sh`. It must pass first.
2. Start the app with `pnpm -C apps/desktop tauri dev` (a debug build) or from an installed build.
   Note which.
3. Files, all of your own drawings (no customer drawings), in a folder you can delete afterwards:
   - **P1**: a phone photo of a printed A4 or A3 drawing, taken straight from above, phone held
     upright (portrait) although the sheet is landscape, so the photo needs a quarter turn.
   - **P2**: a photo of the same sheet taken at a clear slant (about 20 to 30 degrees), whole sheet
     in the picture, on a darker table.
   - **P3**: the same as a HEIC file (iPhone default format), if you have an iPhone.
   - **S1**: a scan of a drawing as PNG or TIFF at 300 dpi.
   - **S2**: a TIFF with two or more pages (most scanners can save several pages into one TIFF).
4. A ruler and a printer, for section 9.

Write down: date, tester, operating system and version, DimoMark version and build, phone model
(only in your notes, not in any file you add to the corpus).

## 1. Open a phone photo

| Step | Do | Expected |
|---|---|---|
| 1.1 | **New project** with P1 | The sheet shows as the phone showed it (EXIF orientation honoured); the setup pane offers **Adjust image** as its first step |
| 1.2 | Draw a box around a dimension with **Place balloon** | No text is found, the balloon is placed and the value field opens |
| 1.3 | **File** > **Info** | Drawing file name is the photo's name, the hash is shown |

## 2. Rotate and straighten

| Step | Do | Expected |
|---|---|---|
| 2.1 | **Adjust image**, **Rotate right** or **Rotate left** until the drawing reads upright | The preview turns in quarter steps |
| 2.2 | Move the **Straighten** slider | The preview turns in small steps; the angle field follows |
| 2.3 | Type `2.5` into **Angle**, then `-50` | 2.5 turns clockwise; -50 becomes -45, the limit the hint names |
| 2.4 | **Reset** | Back to the image as it was opened |

## 3. Corners

| Step | Do | Expected |
|---|---|---|
| 3.1 | Close the editor, **New project** with P2, **Adjust image** > **Corners**, switch on **Correct the corners** | The four handles sit on the sheet corners, or at the image corners with the note that the edge was not found |
| 3.2 | Drag each handle exactly onto a sheet corner | The preview shows a rectangular sheet with straight title block lines |
| 3.3 | Go to **Rotate** | Angle field and slider are off, the hint says the corners straighten the sheet |

## 4. Trim and paper

| Step | Do | Expected |
|---|---|---|
| 4.1 | **Trim**: drag the corners of the rectangle, then the rectangle itself | Only the inside stays; **Keep all** brings back the whole image |
| 4.2 | **Paper**: choose the paper of the printed drawing and its orientation | The preview has the proportions of that paper |
| 4.3 | Choose **Custom size**, type 200 and 100 | A sheet of 2 to 1 |
| 4.4 | Set the right paper again, switch on **Clean up** | Grey image, white background, stronger lines |

## 5. Apply, undo and balloons

| Step | Do | Expected |
|---|---|---|
| 5.1 | **Apply** | The drawing shows the adjusted sheet; one ⌘Z brings back the old one, ⇧⌘Z the new one |
| 5.2 | Place three balloons and type their values | Limits as for a PDF drawing |
| 5.3 | **Adjust image**, change only **Clean up**, **Apply** | The balloons stay on their features |
| 5.4 | **Adjust image**, **Trim** so that one balloon lies outside, **Apply** | DimoMark asks first; **Apply anyway** puts the balloon on the sheet edge; ⌘Z brings it back |
| 5.5 | Save, close, open the project again, **Adjust image** | The settings of 5.3 are still there and can be changed |

## 6. Measure and reports

| Step | Do | Expected |
|---|---|---|
| 6.1 | Add one part, enter a value for each balloon | Pass and fail as for a PDF drawing |
| 6.2 | Export the inspection report PDF | Drawing file name and fingerprint are those of the original photo |

## 7. Scans and TIFF

| Step | Do | Expected |
|---|---|---|
| 7.1 | **New project** with S1, export the ballooned PDF without adjusting | The PDF page has the size of the scanned paper |
| 7.2 | **New project** with S2 | One sheet per TIFF page, each with its own **Adjust image** settings |

## 8. HEIC and other files

| Step | Do | Expected |
|---|---|---|
| 8.1 | **New project** with P3 | Refused with the HEIC message that explains the iPhone setting; nothing else changes |

## 9. Ballooned PDF on paper

| Step | Do | Expected |
|---|---|---|
| 9.1 | Export the ballooned PDF of the project from section 5 | The page has the chosen paper size; the sheet is straight and trimmed; balloons in place |
| 9.2 | Print it at 100 % (no fit to page), measure a long dimension line with the ruler | Within about 1 % of the same line on the original paper drawing |
| 9.3 | Export again without a change, compare the files (`cmp` or a checksum) | Byte identical |

## 10. User guide

| Step | Do | Expected |
|---|---|---|
| 10.1 | Follow [Drawings from photos and scans](../user/en/photos.md) with a new photo | Everything it says matches the app |
| 10.2 | Read the German page [Zeichnungen aus Fotos und Scans](../user/de/photos.md) | Same, and the German words match the app |

## Result

| Area | Pass | Notes |
|---|---|---|
| 1 Open a phone photo | | |
| 2 Rotate and straighten | | |
| 3 Corners | | |
| 4 Trim and paper | | |
| 5 Apply, undo and balloons | | |
| 6 Measure and reports | | |
| 7 Scans and TIFF | | |
| 8 HEIC | | |
| 9 Ballooned PDF on paper | | |
| 10 User guide | | |

Report every failure as an issue with the step number, what happened and the platform.
