# What's new in StitchFoundry

Release notes for each version, newest first, written for the people who use StitchFoundry. The
release workflow copies a version's section into its GitHub release (`docs/releasing.md`).

## 0.15.0

Everything since 0.14.0. This is the first full release of 0.15: it has everything in the two test releases (0.15.0-rc.1 and rc.2) below. Please test-sew designs before using them on a garment that matters.

StitchFoundry is pre-release embroidery software: test-sew every design on scrap fabric first.

### Hoops and machines
- **Hoop editor.** Make your own hoops (size, shape, corner radius and frame) and attach them to your machines.
- **Machine profiles and My Machine.** Edit a machine's profile, including which hoops it owns, choose the save format per machine, and make it the default for Save and Export.
- **Start a design in the hoop.** Pick machine, hoop and fabric first; new designs remember the last hoop you used on that machine. The Home screen hoop tiles open a ready design.
- **Warning when a design is bigger than the hoop,** and **Fit to Hoop**: one action that shrinks the whole design (aspect ratio locked) until it fits the sewing field.
- **Hoop orientation test** and a per-machine export rotation, so designs come out the way the hoop sits in your machine. Test-sew it on scrap first.
- **Quick rotate, flip and resize** commands for the selection (buttons come in a later build).

### Lettering and art
- **Multi-line lettering** with font, left, centre or right alignment, line spacing and a different size for each line, in Add Words and in Make a Design. Every font is drawn in its own style with a live preview of your own words.
- **Clipart:** searchable sets and 20 new original motifs. **Monogram frames:** 11 new decorative templates (20 in the gallery).
- **Font editor.** Open a font by name or folder as a font document and remove glyphs.

### Embroidery
- **Jump and trim counts** per colour in the design details.
- **Stitch advice** (needle, stabilizer and hints by fabric) and plain-language "too stiff" density advice, translated into all supported languages. The advisor screens come in a later build.

### Account and help
- The **Foundry Account** is now the only AI sign-in. The "use my own API key" option is gone.
- **Learn** button with a lesson list loaded from stitchfoundry.app. **Report a Bug** and **Suggest a Feature** open the web forms.
- **Send to my phone** (cloud side): private links that last 24 hours.
- **Founding members** get a numbered badge and a one-time bonus. There is a daily sign-up cap with a next-day queue, and a per-user daily free AI limit.

### Fixes
- Scaling lettering scales each line's own size.
- A very small satin split length is raised to a safe minimum, and very dense plain fills, run, zigzag and satin objects are capped, so a stray setting can't make a runaway stitch count.
- Embroidery file names can no longer become reserved Windows names (like CON or PRN).
- A damaged XXX file can no longer overflow on 32-bit builds, and the bug-report queue numbering can't overflow.
- Customer-facing text says StitchFoundry everywhere; old links and workflow references were updated after the organisation move; a yanked dependency was replaced.

### What has and hasn't been checked
- **Checked in software:** every hoop with every rotate, flip and resize is written to DST and PES and read back; 600 of 600 combinations stay inside the sewing field and agree between the two formats (`docs/qa/addon-40-report.md`).
- **Not done yet:** the same check for other file formats, and a real sew-out of the hoop orientation test. Test-sew on scrap first.
- Buttons for quick rotate, flip and resize, and the Stitch advisor screens, come in a later build.

### Not in this release
- The hoop-first workspace (an overlay toggle, zoom presets and embroidery-first toolbars) is coming in the next release.

## 0.15.0-rc.2 (test release)

Everything since 0.15.0-rc.1. A test release: please tell us what doesn't work, and test-sew designs before using them on a garment that matters.

### Hoops and machines
- **Hoop editor.** Make your own hoops (size, shape, corner radius and frame) and attach them to your machines.
- **Machine profiles.** Edit a machine's profile, including which hoops it owns, and keep one hoop picked per machine.
- **Pick from the hoops you own.** The hoop list can show only the hoops on your machine, and a new design starts in the last hoop you used on that machine.
- **Warning when a design is bigger than the hoop,** and **Fit to Hoop**: one action that shrinks the whole design (aspect ratio locked) until it fits the sewing field.
- **Hoop orientation test** and a per-machine export rotation, so designs come out the way the hoop sits in your machine. Test-sew it on scrap first.
- **Quick rotate, flip and resize** commands for the selection (buttons come in a later build).

### Embroidery
- **Jump and trim counts** per colour in the design details.
- **Stitch advice** (needle, stabilizer and hints by fabric) and "too stiff" density advice, in the engine; screens for the advisor come later.

### Not in this build yet
- The full hoop QA matrix (every hoop times every rotate, flip and resize, in every file format) is still being built. The release notes for 0.15.0 will say when it has passed.
- Buttons for quick rotate, flip and resize, and the Stitch advisor screens.

## 0.15.0-rc.1 (test release)

Everything since 0.14.0. A test release: please tell us what doesn't work, and test-sew designs before using them on a garment that matters.

### Lettering
- **More than one line.** Add Words now takes several lines of text; press Enter for a new line.
- **See the font before you pick it.** One font picker everywhere you choose a font: each font is drawn in its own style, with a live preview of your own words.
- **Line up your lines:** left, centre or right (centre by default).
- **Line spacing** you can adjust, and **a different size for each line.**

### Fixes
- Embroidery file names can no longer become reserved Windows names (like CON or PRN).
- Very dense run, zigzag and satin objects are capped, so a stray setting can't make a runaway stitch count.
- The "before" picture for a file that can't be read is always the placeholder.
- Customer-facing text says StitchFoundry everywhere.

### Not in this build yet
- Custom hoops: the engine is in, but the screens to make and attach your own hoop come in a later release.

## 0.14.0

The first full release of 0.14. It has everything in the 0.14.0-rc.1 test release below (Simple
mode, Online Orders, 39 built-in fonts, Save for a Memory Card and more), plus:

- **Open in StitchFoundry from MakerFoundry**: when a design can't be opened, the web app now
  shows MakerFoundry's own explanation, and the link is cleared from the address bar.
- **Help › Suggest a Feature…** lets you send us an idea, next to Report a Bug.
- **The web app** no longer stops when it checks for updates.
- **Drawing and text**, from the VectorCraft editor StitchFoundry is built on: steadier Shape
  Builder and Join, angle constraints for arrow keys and Shift-drags, better Character and
  Paragraph panels, and SVG, PDF and EPS files that keep more of what you made.

As always, test-sew a design before using it on a garment that matters.

## 0.14.0-rc.1 (test release)

Everything since 0.13.1-rc.2. A test release: please tell us what doesn't work, and test-sew
designs before using them on a garment that matters.

### Easier for beginners

- **Simple mode for people new to computers.** Save for My Machine finds the USB stick you have
  plugged in and saves straight onto it, with one big button per stick when there are several.
  The bottom bar shows See It All, Closer and Farther instead of numbers. Add a Picture opens your
  Pictures folder and puts the picture straight in. Threads are called by colour ("Red", "Sky
  Blue") instead of codes, and sewing times read "7 min" or "4 h 31 min".
- **Light or dark screen**, chosen next to Text size.
- **Big text sizes work everywhere.** Windows taller than the screen scroll, so Done and Next are
  always in view.
- **Plainer words** on Home and in the hints.
- **Make a Design** no longer stops when no hoop or machine is chosen: it uses the everyday 5×7
  hoop. Words too long for the hoop are made smaller to fit, and it says so. A very big design
  says how long it will take to sew.
- **Nine sample designs**, shown as pictures on Home.
- **The app tells you when a newer StitchFoundry is out** (a card on Home, and Help › Check for
  Updates).

### Online orders (new)

- **Embroidery › Online Orders…** reads orders from your Shopify or WooCommerce shop, or a CSV
  export from Etsy or any other shop. It makes each personalized item into a design: the
  customer's name or text, in the font and thread colour your rules choose, fitted to a template.
  Each item also gets a job order, its machine files and a worksheet with a barcode.
- Each order line is made once. It can check every 15 minutes while the window is open.
- **Find a job** by scanning a worksheet's barcode, or by typing the job or order number.
- Setup guide, step by step for each shop: [docs/online-orders.md](https://github.com/Stitchwright/StitchFoundry/blob/main/docs/online-orders.md).

### Fonts

- **39 built-in embroidery fonts** (the Foundry fonts): scripts, serifs, sans, display, western,
  gothic and a 5 mm font for small text, made from openly licensed typefaces. Together with the
  131 free fonts a click away (Get Embroidery Fonts), that's 170. The new fonts were checked on
  screen; please sew a test before relying on them.
- **Kerning classes** in the font creator: kern a letter against a whole group at once.

### Saving and sharing

- **Save for a Memory Card**: pick your machine's brand and it uses the right format and limits.
- **Eject USB Stick** after Save for My Machine. On macOS, StitchFoundry can be the default app
  for machine files.
- **Watch it sew as a video**: save the sew-out as a looping GIF or an MP4.
- **Send to StitchShop**: the design, its preview and details go to your StitchShop inbox (and
  Take It Back).
- **Free web viewer**: open a design from a link in the browser, with Edit It Here and Get
  StitchFoundry Free.
- **Husqvarna HUS** and **Janome SEW** are read and written, and **Pfaff VIP** is read (all
  experimental).
- **Machine files checked against an independent reader**, with fixes, and a warning when a
  format can't keep something.

### Photos and artwork

- **Remove Background** for photos, with **Select Subject**, a Cut-Out Brush and a before-and-after
  preview. Make a Design cuts a photo's subject out first (a setting).
- **Magnetic Lasso**: draw round part of a photo and the shape follows its edges.
- **Click to Stitch** can take every area of a colour, not only the one clicked.
- **Auto-Digitize**: no fabric showing through between colours, and thin dark lines sewn as runs
  on top.
- **Borders round digitized photos.**
- **New photo stitch styles**: sepia, black and white, and colour tiles.
- **DWG drawings** open on the desktop, and **binary DXF** opens. The **Fill Region** tool makes a
  fill from a click between loose lines.
- **Warp and Distort effects** are sewn as the art shows.
- **Design from Prompt**: an Effort setting with the cost shown before and after. Thorough
  drawings look up what they draw and are checked as they stitch.

### Projects and wizards

- **More in-the-hoop projects**: zipper pouch, pencil case, card holder, phone case, passport
  holder, book cover, key fob, luggage tag and bookmark. Projects come in more sizes, with
  rounded, scalloped or round edges.
- **Endless Border**, **Spiro**, **Repeat Design**, **Quilt Background** and **Family Tree**
  wizards.

### Stitching

- **New stitch effects**: Make Eyelets (the hole, a stop to cut it, a satin binding), Make
  Lacework (free-standing lace), StumpWork pieces, Wave Fill, ragged edges on fills, and
  blackwork and candlewick motifs.
- **Recolour Threads**: Lighter, Darker and Tinted, in one step.
- **Remove Short Stitches** from any machine file.
- **Satin columns** can be denser in some places than others, or spaced unevenly on purpose for
  texture.

### From VectorCraft (the drawing app StitchFoundry is built on)

About 200 improvements to drawing, type and file import:
- Objects on Path.
- A new Simplify slider.
- Measurement labels while drawing.
- Rotate View dial.
- Better rulers in inches.
- More Sketch and Artistic effects.
- Live text kept in placed PDFs.
- Saved workspaces keep their panel widths.
- Many fixes.

### Not in this build

- **HEIC (iPhone) photos** are left out of official builds for now, because of the HEVC patents
  in some countries. Convert them to JPEG first.
