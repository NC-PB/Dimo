# Project and report details

**File > Info** holds the data that identifies the part, the drawing and the inspection report;
the side panel of the Measure view shows it too. With nothing selected, the properties pane of the
drawing shows a summary with the way to File > Info. The data is stored in the project and printed in the header and the signature block
of the reports.

Every field is saved when you leave it or press Enter. Each change is one step of undo
(⌘Z), like every other change of the project. If Dimo refuses a value, the field shows the
old value again and a message above the drawing tells why.

## Part

| Field | Meaning |
|---|---|
| Part number | Number of the part as the customer calls it |
| Part name | Name of the part |
| Customer | Who ordered the part |
| Order reference | Order or purchase reference |

## Drawing

| Field | Meaning |
|---|---|
| Revision | Revision label of the drawing, for example `B`. Dimo does not read it from the title block, so type it from the drawing |
| Drawing file | File name of the imported drawing (cannot be edited) |
| SHA-256 | Fingerprint of the drawing file (cannot be edited). **Copy** puts it on the clipboard. The report prints it, so anyone can check that a report belongs to this exact drawing |

The revision label belongs to the drawing in the project. Dimo keeps it with the project and
prints it on every report.

## Report

| Field | Meaning |
|---|---|
| Report number | Number of the inspection report |
| Report date | Date of the report. Leave it empty and the first report export enters today's date; or choose **Today** to fill it in now, or type a date. Once stored, the date is part of the project and does not change by itself |
| Company | Name of your company. See below |
| Inspected by | Who inspected the parts |
| Approved by | Who approved the report |
| Remarks | Free text printed on the report |

"Inspected by" and "Approved by" print in the signature block of the inspection report. The date
of "Inspected by" is the report date; "Approved by" gets no date, you write it by hand. This is
separate from the inspector of each sample, which you set in the sample panel
([Entering results](measuring.md)). If the report date is empty when you export a check sheet or an inspection report,
Dimo enters today's date first, as one change you can undo ([Exports](exports.md)).

Empty fields stay empty on the report. A new project starts with the company name from **Options** and with the user name of **Options** (or of your system) as inspector. Dimo does not change an existing project when you open it.

### Company name

Enter your company name once in **File** > **Options**, under **You and your company**
([File page, options and About](settings.md)). A new project starts with that name in the field, as part of its first
state (it is not a step of undo and does not mark the project as changed). Changing the setting
later does not change existing projects; you can edit the field in each project.
