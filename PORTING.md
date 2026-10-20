# Porting checklist

Status: `DONE` (ported, Rust location given), `N/A` (not needed, reason given), `TODO`.
Designer files (`*.Designer.cs`) belong to the form row of the same name: layout/captions/defaults must be mirrored.
Vendored libraries are not listed (see PLAN.md §6).

## `AssemblyInfo.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|

## `AutoUpdate/GitHub.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class OnlineVersion` | N/A | removed: no update check / phone-home |
| `field html_url` | N/A | removed: no update check / phone-home |
| `field tag_name` | N/A | removed: no update check / phone-home |
| `field name` | N/A | removed: no update check / phone-home |
| `field prerelease` | N/A | removed: no update check / phone-home |
| `field assets` | N/A | removed: no update check / phone-home |
| `class VersionAsset` | N/A | removed: no update check / phone-home |
| `field browser_download_url` | N/A | removed: no update check / phone-home |
| `property Url` | N/A | removed: no update check / phone-home |
| `property IsValid` | N/A | removed: no update check / phone-home |
| `property IsPreRelease` | N/A | removed: no update check / phone-home |
| `property VersionName` | N/A | removed: no update check / phone-home |
| `method ToString` | N/A | removed: no update check / phone-home |
| `delegate void` | N/A | event of the disabled update check |
| `field NewVersion` | N/A | event of the disabled update check |
| `field Updating` | N/A | update disabled |
| `method CheckVersion` | N/A | network update check disabled (PLAN section 6); check_version() returns false |
| `method AsyncCheckVersion` | N/A | network disabled (PLAN section 6) |
| `method AsyncDownloadDB` | N/A | network disabled (PLAN section 6) |
| `method CheckVersionFromSite` | N/A | removed: no update check / phone-home |
| `property installer` | N/A | removed: no update check / phone-home |
| `field client` | N/A | update disabled |
| `method DownloadUpdateA` | N/A | update disabled |
| `method disposeclient` | N/A | update disabled |
| `method ApplyUpdateEXE` | N/A | Windows installer, update disabled |
| `method InitUpdate` | N/A | removed: no update check / phone-home |
| `method InitFlags` | N/A | removed: no update check / phone-home |
| `method CleanupOldVersion` | N/A | deletes *.todelete below the cwd; Windows installer leftovers |

## `AutoUpdate/NewVersionForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class NewVersionForm` | N/A | removed: no update check / phone-home |
| `field mDownloadUrl` | N/A | removed: no update check / phone-home |
| `field mPageUrl` | N/A | removed: no update check / phone-home |
| `field mClosing` | N/A | removed: no update check / phone-home |
| `ctor NewVersionForm` | N/A | removed: no update check / phone-home |
| `method CreateAndShowDialog` | N/A | removed: no update check / phone-home |
| `method BtnUpdate_Click` | N/A | removed: no update check / phone-home |
| `method dprog` | N/A | removed: no update check / phone-home |
| `method dcomplete` | N/A | removed: no update check / phone-home |
| `method NewVersionForm_FormClosing` | N/A | removed: no update check / phone-home |
| `method BtnWebsite_Click` | N/A | removed: no update check / phone-home |

## `Autotrace/Autotrace.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class Autotrace` | DONE | crates/services/src/autotrace.rs |
| `property TempPath` | DONE | crates/services/src/autotrace.rs |
| `method CleanupTmpFolder` | DONE | crates/services/src/autotrace.rs |
| `method BitmapToSvgDocument` | DONE | crates/services/src/autotrace.rs `bitmap_to_svg_string` + `parse_svg` in image_processor/job.rs (no `SvgDocument` type; only the preview uses it) |
| `method BitmapToSvgString` | DONE | crates/services/src/autotrace.rs |
| `method ToHexString` | N/A | unused private helper in C# |
| `method ExecuteAutotrace` | DONE | crates/services/src/autotrace.rs |

## `Autotrace/AutotraceTest.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class AutotraceTest` | N/A | developer test form, not ported |
| `ctor AutotraceTest` | N/A | developer test form, not ported |
| `field source` | N/A | developer test form, not ported |
| `delegate void` | N/A | developer test form, not ported |
| `field exiting` | N/A | developer test form, not ported |
| `field refresh` | N/A | developer test form, not ported |
| `method DoTheWork` | N/A | developer test form, not ported |
| `method OnClosing` | N/A | developer test form, not ported |
| `method DoSetImage` | N/A | developer test form, not ported |
| `method pictureBox1_Click` | N/A | developer test form, not ported |
| `method TrackChange` | N/A | developer test form, not ported |
| `method PreviewCenterline` | N/A | developer test form, not ported |
| `method ExecuteCommand` | N/A | developer test form, not ported |
| `method CheckedChanged` | N/A | developer test form, not ported |

## `CSV/CsvDictionary.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class CsvDictionary` | DONE | crates/base/src/csv/dictionary.rs |
| `field mD` | DONE | crates/base/src/csv/dictionary.rs |
| `field mLen` | DONE | crates/base/src/csv/dictionary.rs |
| `ctor CsvDictionary` | DONE | crates/base/src/csv/dictionary.rs |
| `method GetItem` | DONE | crates/base/src/csv/dictionary.rs |

## `CSV/FormattedMessage.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class FormattedMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `enum DTFormat` | DONE | crates/base/src/csv/formatted_message.rs |
| `field list` | DONE | crates/base/src/csv/formatted_message.rs |
| `field i` | DONE | crates/base/src/csv/formatted_message.rs |
| `ctor FormattedMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `ctor FormattedMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ToStringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ToMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method FromStringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Append` | DONE | crates/base/src/csv/formatted_message.rs |
| `method BeginRead` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadBool` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadBool` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadInt` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadInt` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadLong` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadLong` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDecimal` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDecimal` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadFloat` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadFloat` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDouble` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDouble` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadString` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadString` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadTimeSpan` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadTimeSpan` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDateTime` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDateTime` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDateTime` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ReadDateTime` | DONE | crates/base/src/csv/formatted_message.rs |
| `method UnixToDotNet` | DONE | crates/base/src/csv/formatted_message.rs |
| `method DotNetToUnix` | DONE | crates/base/src/csv/formatted_message.rs |
| `class StringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `field esc` | DONE | crates/base/src/csv/formatted_message.rs |
| `field sep` | DONE | crates/base/src/csv/formatted_message.rs |
| `field phl` | DONE | crates/base/src/csv/formatted_message.rs |
| `field orig` | DONE | crates/base/src/csv/formatted_message.rs |
| `field list` | DONE | crates/base/src/csv/formatted_message.rs |
| `ctor StringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `ctor StringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `ctor StringList` | DONE | crates/base/src/csv/formatted_message.rs |
| `method ToMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method FromMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method SplitMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method EndReached` | DONE | crates/base/src/csv/formatted_message.rs |
| `method SeparatorFound` | DONE | crates/base/src/csv/formatted_message.rs |
| `method substring` | DONE | crates/base/src/csv/formatted_message.rs |
| `method BuildMessage` | DONE | crates/base/src/csv/formatted_message.rs |
| `method sput` | DONE | crates/base/src/csv/formatted_message.rs |
| `method sget` | DONE | crates/base/src/csv/formatted_message.rs |
| `method nullify` | DONE | crates/base/src/csv/formatted_message.rs |
| `method unnullify` | DONE | crates/base/src/csv/formatted_message.rs |
| `method escape` | DONE | crates/base/src/csv/formatted_message.rs |
| `method unescape` | DONE | crates/base/src/csv/formatted_message.rs |
| `method IndexOf` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method Insert` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method RemoveAt` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method Add` | DONE | crates/base/src/csv/formatted_message.rs |
| `method Clear` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method Contains` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method CopyTo` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method Remove` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |
| `method GetEnumerator` | N/A | IList plumbing unused by C# callers; Rust StringList exposes len/get/add |

## `Clipper/Clipper.cs` (WP-B)

| member | status | Rust location / reason |
|---|---|---|
| `struct DoublePoint` | DONE | `crates/clipper/src/types.rs` `DoublePoint` |
| `field X` | DONE | `DoublePoint.x` |
| `field Y` | DONE | `DoublePoint.y` |
| `ctor DoublePoint` | DONE | `DoublePoint::new` |
| `ctor DoublePoint` | N/A | Copy type, no copy ctor |
| `ctor DoublePoint` | DONE | `DoublePoint::from_int_point` |
| `class PolyTree` | DONE | `crates/clipper/src/polytree.rs` `PolyTree` (arena, node 0 = tree) |
| `field m_AllPolys` | DONE | `PolyTree.all_polys` |
| `method Clear` | DONE | `PolyTree::clear` |
| `method GetFirst` | DONE | `PolyTree::get_first` |
| `class PolyNode` | DONE | `crates/clipper/src/polytree.rs` `PolyNode` |
| `field m_Parent` | DONE | `PolyNode.parent` |
| `field m_polygon` | DONE | `PolyNode.polygon` / `contour()` |
| `field m_Index` | DONE | `PolyNode.index` |
| `field m_jointype` | N/A | only used by ClipperOffset: kept in `OffsetNode.join_type` (offset.rs) |
| `field m_endtype` | N/A | only used by ClipperOffset: kept in `OffsetNode.end_type` (offset.rs) |
| `field m_Childs` | DONE | `PolyNode.childs` / `childs()` |
| `method IsHoleNode` | DONE | `PolyTree::is_hole` |
| `method AddChild` | DONE | `PolyTree::add_child` |
| `method GetNext` | DONE | `PolyTree::get_next` |
| `method GetNextSiblingUp` | DONE | `PolyTree::get_next_sibling_up` |
| `property IsOpen` | DONE | `PolyNode.is_open` / `is_open()` |
| `struct Int128` | DONE | `crates/clipper/src/types.rs` `Int128` |
| `field hi` | DONE | `Int128.hi` |
| `field lo` | DONE | `Int128.lo` |
| `ctor Int128` | DONE | `Int128::from_i64` |
| `ctor Int128` | DONE | `Int128::from_parts` |
| `ctor Int128` | N/A | Copy type |
| `method IsNegative` | DONE | `Int128::is_negative` |
| `field operator` | N/A | derived `PartialEq`/`Eq` (Equals/hash not needed) |
| `method operator !=` | N/A | derived `PartialEq`/`Eq` (Equals/hash not needed) |
| `method Equals` | N/A | derived `PartialEq`/`Eq` (Equals/hash not needed) |
| `method GetHashCode` | N/A | derived `PartialEq`/`Eq` (Equals/hash not needed) |
| `method operator >` | N/A | operator never used by the C# library (only ==) |
| `method operator <` | N/A | operator never used by the C# library (only ==) |
| `method operator +` | DONE | `Int128::add` |
| `method operator -` | DONE | `Int128::sub` |
| `method operator -` | DONE | `Int128::neg` |
| `method double` | DONE | `Int128::to_f64` |
| `field shift64` | DONE | const `SHIFT64` in `Int128::to_f64` |
| `method Int128Mul` | DONE | `Int128::mul` |
| `struct IntPoint` | DONE | `IntPoint` |
| `field X` | DONE | `IntPoint.x` |
| `field Y` | DONE | `IntPoint.y` |
| `field Z` | N/A | use_xyz not defined |
| `ctor IntPoint` | N/A | use_xyz not defined |
| `ctor IntPoint` | N/A | use_xyz not defined |
| `ctor IntPoint` | N/A | use_xyz not defined |
| `ctor IntPoint` | N/A | use_xyz not defined |
| `ctor IntPoint` | DONE | `IntPoint::new` |
| `ctor IntPoint` | DONE | `IntPoint::from_f64` (truncating cast) |
| `ctor IntPoint` | N/A | Copy type |
| `field operator` | N/A | derived `PartialEq`/`Eq`/`Hash` |
| `method operator !=` | N/A | derived `PartialEq`/`Eq`/`Hash` |
| `method Equals` | N/A | derived `PartialEq`/`Eq`/`Hash` |
| `method GetHashCode` | N/A | derived `PartialEq`/`Eq`/`Hash` |
| `struct IntRect` | DONE | `IntRect` |
| `field left` | DONE | `IntRect.left` |
| `field top` | DONE | `IntRect.top` |
| `field right` | DONE | `IntRect.right` |
| `field bottom` | DONE | `IntRect.bottom` |
| `ctor IntRect` | DONE | `IntRect::new` |
| `ctor IntRect` | N/A | Copy type |
| `enum ClipType` | DONE | `ClipType` (types.rs) |
| `enum PolyType` | DONE | `PolyType` (types.rs) |
| `enum PolyFillType` | DONE | `PolyFillType` (types.rs) |
| `enum JoinType` | DONE | `JoinType` (types.rs) |
| `enum EndType` | DONE | `EndType` (types.rs) |
| `enum EdgeSide` | DONE | `crates/clipper/src/clipper.rs` `EdgeSide` |
| `enum Direction` | DONE | `crates/clipper/src/clipper.rs` `Direction` |
| `class TEdge` | DONE | `crates/clipper/src/clipper.rs` `TEdge` |
| `field Bot` | DONE | `TEdge.bot` (arena index) |
| `field Curr` | DONE | `TEdge.curr` (arena index) |
| `field Top` | DONE | `TEdge.top` (arena index) |
| `field Delta` | DONE | `TEdge.delta` (arena index) |
| `field Dx` | DONE | `TEdge.dx` (arena index) |
| `field PolyTyp` | DONE | `TEdge.poly_typ` (arena index) |
| `field Side` | DONE | `TEdge.side` (arena index) |
| `field WindDelta` | DONE | `TEdge.wind_delta` (arena index) |
| `field WindCnt` | DONE | `TEdge.wind_cnt` (arena index) |
| `field WindCnt2` | DONE | `TEdge.wind_cnt2` (arena index) |
| `field OutIdx` | DONE | `TEdge.out_idx` (arena index) |
| `field Next` | DONE | `TEdge.next` (arena index) |
| `field Prev` | DONE | `TEdge.prev` (arena index) |
| `field NextInLML` | DONE | `TEdge.next_in_lml` (arena index) |
| `field NextInAEL` | DONE | `TEdge.next_in_ael` (arena index) |
| `field PrevInAEL` | DONE | `TEdge.prev_in_ael` (arena index) |
| `field NextInSEL` | DONE | `TEdge.next_in_sel` (arena index) |
| `field PrevInSEL` | DONE | `TEdge.prev_in_sel` (arena index) |
| `class IntersectNode` | DONE | `IntersectNode` |
| `field Edge1` | DONE | `edge1` |
| `field Edge2` | DONE | `edge2` |
| `field Pt` | DONE | `pt` |
| `class MyIntersectNodeSort` | DONE | closure in `Clipper::fixup_intersection_order` (sorted with `sort.rs` introsort = .NET List.Sort) |
| `method Compare` | DONE | closure in `fixup_intersection_order` |
| `class LocalMinima` | DONE | `LocalMinima` (arena) |
| `field Y` | DONE | `y` |
| `field LeftBound` | DONE | `left_bound` |
| `field RightBound` | DONE | `right_bound` |
| `field Next` | DONE | `next` |
| `class Scanbeam` | DONE | `Clipper.scanbeam`: sorted `Vec<CInt>`, same pop/insert behaviour as the linked list |
| `field Y` | N/A | folded into `scanbeam` Vec |
| `field Next` | N/A | folded into `scanbeam` Vec |
| `class Maxima` | DONE | `Maxima` (arena) |
| `field X` | DONE | `x` |
| `field Next` | DONE | `next` |
| `field Prev` | DONE | `prev` |
| `class OutRec` | DONE | `OutRec` (arena `poly_outs`) |
| `field Idx` | DONE | `OutRec.idx` |
| `field IsHole` | DONE | `OutRec.is_hole` |
| `field IsOpen` | DONE | `OutRec.is_open` |
| `field FirstLeft` | DONE | `OutRec.first_left` |
| `field Pts` | DONE | `OutRec.pts` |
| `field BottomPt` | DONE | `OutRec.bottom_pt` |
| `field PolyNode` | DONE | `OutRec.poly_node` |
| `class OutPt` | DONE | `OutPt` (arena `out_pts`) |
| `field Idx` | DONE | `OutPt.idx` |
| `field Pt` | DONE | `OutPt.pt` |
| `field Next` | DONE | `OutPt.next` |
| `field Prev` | DONE | `OutPt.prev` |
| `class Join` | DONE | `Join` |
| `field OutPt1` | DONE | `Join.out_pt1` |
| `field OutPt2` | DONE | `Join.out_pt2` |
| `field OffPt` | DONE | `Join.off_pt` |
| `class ClipperBase` | DONE | `crates/clipper/src/clipper.rs` `Clipper` (ClipperBase merged: no inheritance in Rust) |
| `field horizontal` | DONE | `HORIZONTAL` |
| `field Skip` | DONE | `SKIP` |
| `field Unassigned` | DONE | `UNASSIGNED` |
| `field tolerance` | DONE | `TOLERANCE` |
| `method near_zero` | DONE | `near_zero` |
| `field loRange` | N/A | use_int32 not defined |
| `field hiRange` | N/A | use_int32 not defined |
| `field loRange` | DONE | `LO_RANGE` (types.rs) |
| `field hiRange` | DONE | `HI_RANGE` (types.rs) |
| `field m_MinimaList` | DONE | `Clipper.minima_list` + arena `minima` |
| `field m_CurrentLM` | DONE | `Clipper.current_lm` |
| `field m_edges` | DONE | `Clipper.edges` (single arena instead of List<List<TEdge>>) |
| `field m_Scanbeam` | DONE | `Clipper.scanbeam` |
| `field m_PolyOuts` | DONE | `Clipper.poly_outs` |
| `field m_ActiveEdges` | DONE | `Clipper.active_edges` |
| `field m_UseFullRange` | DONE | `Clipper.use_full_range` |
| `field m_HasOpenPaths` | DONE | `Clipper.has_open_paths` |
| `method Swap` | N/A | `std::mem::swap` |
| `method IsHorizontal` | N/A | inline `delta.y == 0` |
| `method PointIsVertex` | DONE | `Clipper::point_is_vertex` |
| `method PointOnLineSegment` | DONE | `Clipper::point_on_line_segment` |
| `method PointOnPolygon` | DONE | `Clipper::point_on_polygon` |
| `method SlopesEqual` | DONE | `slopes_equal_edges` |
| `method SlopesEqual` | DONE | `slopes_equal3` |
| `method SlopesEqual` | DONE | `slopes_equal4` |
| `ctor ClipperBase` | DONE | `Clipper::new` |
| `method Clear` | DONE | `Clipper::clear` |
| `method DisposeLocalMinimaList` | DONE | folded into `Clipper::clear` (arena) |
| `method InitEdge` | DONE | `Clipper::init_edge` |
| `method InitEdge2` | DONE | `Clipper::init_edge2` |
| `method FindNextLocMin` | DONE | `Clipper::find_next_loc_min` |
| `method ProcessBound` | DONE | `Clipper::process_bound` |
| `method AddPath` | DONE | `Clipper::add_path` |
| `method AddPaths` | DONE | `Clipper::add_paths` |
| `method Pt2IsBetweenPt1AndPt3` | DONE | `Clipper::pt2_is_between_pt1_and_pt3` |
| `method SetDx` | DONE | `Clipper::set_dx` |
| `method InsertLocalMinima` | DONE | `Clipper::insert_local_minima` |
| `method PopLocalMinima` | DONE | `Clipper::pop_local_minima` |
| `method ReverseHorizontal` | DONE | `Clipper::reverse_horizontal` |
| `method Reset` | DONE | `Clipper::reset` |
| `method GetBounds` | DONE | `Clipper::get_bounds` |
| `method InsertScanbeam` | DONE | `Clipper::insert_scanbeam` |
| `method PopScanbeam` | DONE | `Clipper::pop_scanbeam` |
| `method LocalMinimaPending` | DONE | `Clipper::local_minima_pending` |
| `method CreateOutRec` | DONE | `Clipper::create_out_rec` |
| `method DisposeOutRec` | DONE | folded into `dispose_all_poly_pts` (arena clear) |
| `method UpdateEdgeIntoAEL` | DONE | `Clipper::update_edge_into_ael` |
| `method SwapPositionsInAEL` | DONE | `Clipper::swap_positions_in_ael` |
| `method DeleteFromAEL` | DONE | `Clipper::delete_from_ael` |
| `class Clipper` | DONE | `Clipper` |
| `field ioReverseSolution` | DONE | `IO_REVERSE_SOLUTION` |
| `field ioStrictlySimple` | DONE | `IO_STRICTLY_SIMPLE` |
| `field ioPreserveCollinear` | DONE | `IO_PRESERVE_COLLINEAR` |
| `field m_ClipType` | DONE | `Clipper.clip_type` |
| `field m_Maxima` | DONE | `Clipper.maxima_head + arena `maxima`` |
| `field m_SortedEdges` | DONE | `Clipper.sorted_edges` |
| `field m_IntersectList` | DONE | `Clipper.intersect_list` |
| `field m_ExecuteLocked` | DONE | `Clipper.execute_locked` |
| `field m_ClipFillType` | DONE | `Clipper.clip_fill_type` |
| `field m_SubjFillType` | DONE | `Clipper.subj_fill_type` |
| `field m_Joins` | DONE | `Clipper.joins` |
| `field m_GhostJoins` | DONE | `Clipper.ghost_joins` |
| `field m_UsingPolyTree` | DONE | `Clipper.using_poly_tree` |
| `delegate void` | N/A | use_xyz not defined |
| `property ZFillFunction` | N/A | use_xyz not defined |
| `ctor Clipper` | DONE | `Clipper::new` |
| `method InsertMaxima` | DONE | `Clipper::insert_maxima` |
| `method Execute` | DONE | `Clipper::execute` (paths, one fill type) |
| `method Execute` | DONE | `Clipper::execute_tree` (PolyTree, one fill type) |
| `method Execute` | DONE | `Clipper::execute_fill` (paths, subj+clip fill) |
| `method Execute` | DONE | `Clipper::execute_tree_fill` (PolyTree, subj+clip fill) |
| `method FixHoleLinkage` | DONE | `Clipper::fix_hole_linkage` |
| `method ExecuteInternal` | DONE | `Clipper::execute_internal` |
| `method DisposeAllPolyPts` | DONE | `Clipper::dispose_all_poly_pts` |
| `method AddJoin` | DONE | `Clipper::add_join` |
| `method AddGhostJoin` | DONE | `Clipper::add_ghost_join` |
| `method SetZ` | N/A | use_xyz not defined |
| `method InsertLocalMinimaIntoAEL` | DONE | `Clipper::insert_local_minima_into_ael` |
| `method InsertEdgeIntoAEL` | DONE | `Clipper::insert_edge_into_ael` |
| `method E2InsertsBeforeE1` | DONE | `Clipper::e2_inserts_before_e1` |
| `method IsEvenOddFillType` | DONE | `Clipper::is_even_odd_fill_type` |
| `method IsEvenOddAltFillType` | DONE | `Clipper::is_even_odd_alt_fill_type` |
| `method IsContributing` | DONE | `Clipper::is_contributing` |
| `method SetWindingCount` | DONE | `Clipper::set_winding_count` |
| `method AddEdgeToSEL` | DONE | `Clipper::add_edge_to_sel` |
| `method PopEdgeFromSEL` | DONE | `Clipper::pop_edge_from_sel` |
| `method CopyAELToSEL` | DONE | `Clipper::copy_ael_to_sel` |
| `method SwapPositionsInSEL` | DONE | `Clipper::swap_positions_in_sel` |
| `method AddLocalMaxPoly` | DONE | `Clipper::add_local_max_poly` |
| `method AddLocalMinPoly` | DONE | `Clipper::add_local_min_poly` |
| `method AddOutPt` | DONE | `Clipper::add_out_pt` |
| `method GetLastOutPt` | DONE | `Clipper::get_last_out_pt` |
| `method SwapPoints` | DONE | `Clipper::swap_points` |
| `method HorzSegmentsOverlap` | DONE | `Clipper::horz_segments_overlap` |
| `method SetHoleState` | DONE | `Clipper::set_hole_state` |
| `method GetDx` | DONE | `Clipper::get_dx` |
| `method FirstIsBottomPt` | DONE | `Clipper::first_is_bottom_pt` |
| `method GetBottomPt` | DONE | `Clipper::get_bottom_pt` |
| `method GetLowermostRec` | DONE | `Clipper::get_lowermost_rec` |
| `method GetOutRec` | DONE | `Clipper::get_out_rec` |
| `method AppendPolygon` | DONE | `Clipper::append_polygon` |
| `method ReversePolyPtLinks` | DONE | `Clipper::reverse_poly_pt_links` |
| `method SwapSides` | DONE | `Clipper::swap_sides` |
| `method SwapPolyIndexes` | DONE | `Clipper::swap_poly_indexes` |
| `method IntersectEdges` | DONE | `Clipper::intersect_edges` |
| `method DeleteFromSEL` | DONE | `Clipper::delete_from_sel` |
| `method ProcessHorizontals` | DONE | `Clipper::process_horizontals` |
| `method ProcessHorizontal` | DONE | `Clipper::process_horizontal` |
| `method GetNextInAEL` | DONE | `Clipper::get_next_in_ael` |
| `method IsMinima` | DONE | `Clipper::is_minima` |
| `method IsMaxima` | DONE | `Clipper::is_maxima` |
| `method IsIntermediate` | DONE | `Clipper::is_intermediate` |
| `method GetMaximaPair` | DONE | `Clipper::get_maxima_pair` |
| `method GetMaximaPairEx` | DONE | `Clipper::get_maxima_pair_ex` |
| `method ProcessIntersections` | DONE | `Clipper::process_intersections` |
| `method BuildIntersectList` | DONE | `Clipper::build_intersect_list` |
| `method EdgesAdjacent` | DONE | `Clipper::edges_adjacent` |
| `method IntersectNodeSort` | DONE | `Clipper::intersect_node_sort` |
| `method FixupIntersectionOrder` | DONE | `Clipper::fixup_intersection_order` |
| `method ProcessIntersectList` | DONE | `Clipper::process_intersect_list` |
| `method Round` | DONE | `types::round` |
| `method TopX` | DONE | `top_x` (free fn) |
| `method IntersectPoint` | DONE | `intersect_point (free fn)` |
| `method ProcessEdgesAtTopOfScanbeam` | DONE | `Clipper::process_edges_at_top_of_scanbeam` |
| `method DoMaxima` | DONE | `Clipper::do_maxima` |
| `method ReversePaths` | DONE | `Clipper::reverse_paths` |
| `method Orientation` | DONE | `Clipper::orientation` |
| `method PointCount` | DONE | `Clipper::point_count` |
| `method BuildResult` | DONE | `Clipper::build_result` |
| `method BuildResult2` | DONE | `Clipper::build_result2` |
| `method FixupOutPolyline` | DONE | `Clipper::fixup_out_polyline` |
| `method FixupOutPolygon` | DONE | `Clipper::fixup_out_polygon` |
| `method JoinPoints` | DONE | `Clipper::join_points` |
| `method PointInPolygon` | DONE | `Clipper::point_in_polygon` |
| `method PointInPolygon` | DONE | `point_in_ring (free fn)` |
| `method Poly2ContainsPoly1` | DONE | `poly2_contains_poly1 (free fn)` |
| `method FixupFirstLefts1` | DONE | `Clipper::fixup_first_lefts1` |
| `method FixupFirstLefts2` | DONE | `Clipper::fixup_first_lefts2` |
| `method FixupFirstLefts3` | DONE | `Clipper::fixup_first_lefts3` |
| `method ParseFirstLeft` | DONE | `Clipper::parse_first_left` |
| `method JoinCommonEdges` | DONE | `Clipper::join_common_edges` |
| `method UpdateOutPtIdxs` | DONE | `Clipper::update_out_pt_idxs` |
| `method DoSimplePolygons` | DONE | `Clipper::do_simple_polygons` |
| `method Area` | DONE | `Clipper::area` |
| `method Area` | DONE | `area_ring (free fn, called with the OutRec pts)` |
| `method Area` | DONE | `area_ring (free fn)` |
| `method SimplifyPolygon` | DONE | `Clipper::simplify_polygon` |
| `method SimplifyPolygons` | DONE | `Clipper::simplify_polygons` |
| `method DistanceSqrd` | DONE | `Clipper::distance_sqrd` |
| `method DistanceFromLineSqrd` | DONE | `Clipper::distance_from_line_sqrd` |
| `method SlopesNearCollinear` | DONE | `Clipper::slopes_near_collinear` |
| `method PointsAreClose` | DONE | `Clipper::points_are_close` |
| `method ExcludeOp` | DONE | `Clipper::exclude_op` |
| `method CleanPolygon` | DONE | `Clipper::clean_polygon` |
| `method CleanPolygons` | DONE | `Clipper::clean_polygons` |
| `method Minkowski` | DONE | `Clipper::minkowski` |
| `method MinkowskiSum` | DONE | `Clipper::minkowski_sum` |
| `method TranslatePath` | DONE | `Clipper::translate_path` |
| `method MinkowskiSum` | DONE | `Clipper::minkowski_sum_paths` |
| `method MinkowskiDiff` | DONE | `Clipper::minkowski_diff` |
| `enum NodeType` | DONE | `NodeType` (clipper.rs) |
| `method PolyTreeToPaths` | DONE | `Clipper::poly_tree_to_paths` |
| `method AddPolyNodeToPaths` | DONE | `Clipper::add_poly_node_to_paths` |
| `method OpenPathsFromPolyTree` | DONE | `Clipper::open_paths_from_poly_tree` |
| `method ClosedPathsFromPolyTree` | DONE | `Clipper::closed_paths_from_poly_tree` |
| `class ClipperOffset` | DONE | `crates/clipper/src/offset.rs` `ClipperOffset` |
| `field m_destPolys` | DONE | `ClipperOffset.dest_polys` |
| `field m_srcPoly` | DONE | `ClipperOffset.src_poly` |
| `field m_destPoly` | DONE | `ClipperOffset.dest_poly` |
| `field m_normals` | DONE | `ClipperOffset.normals` |
| `field m_cos` | DONE | `ClipperOffset.cos` |
| `field m_StepsPerRad` | DONE | `ClipperOffset.steps_per_rad` |
| `field m_lowest` | DONE | `ClipperOffset.lowest` |
| `field m_polyNodes` | DONE | `ClipperOffset.poly_nodes (Vec<OffsetNode>)` |
| `property ArcTolerance` | DONE | `ClipperOffset.arc_tolerance` |
| `property MiterLimit` | DONE | `ClipperOffset.miter_limit` |
| `field two_pi` | DONE | `TWO_PI` |
| `field def_arc_tolerance` | DONE | `DEF_ARC_TOLERANCE` |
| `ctor ClipperOffset` | DONE | `ClipperOffset::new` |
| `method Clear` | DONE | `ClipperOffset::clear` |
| `method Round` | DONE | `types::round` |
| `method AddPath` | DONE | `ClipperOffset::add_path` |
| `method AddPaths` | DONE | `ClipperOffset::add_paths` |
| `method FixOrientations` | DONE | `ClipperOffset::fix_orientations` |
| `method GetUnitNormal` | DONE | `ClipperOffset::get_unit_normal` |
| `method DoOffset` | DONE | `ClipperOffset::do_offset` |
| `method Execute` | DONE | `ClipperOffset::execute` |
| `method Execute` | DONE | `ClipperOffset::execute_tree` |
| `method DoSquare` | DONE | `ClipperOffset::do_square` |
| `method DoMiter` | DONE | `ClipperOffset::do_miter` |
| `method DoRound` | DONE | `ClipperOffset::do_round` |
| `ctor ClipperException` | DONE | `ClipperException` (types.rs) |

## `ColorScheme.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `interface IColorScheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeCADStyle` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeCADDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeBlueLaser` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeRedLaser` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeHacker` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class SchemeNighty` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property IsDark` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `class ColorScheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `enum Scheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `field mDefaultSchemas` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `ctor ColorScheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `method AddSchema` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property CurrentScheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property CurrentSchemeColors` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DarkScheme` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property FormForeColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewText` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewRuler` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGrid` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewGridMinor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewJobRange` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewFirstMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewOtherMovement` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewLaserPower` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCross` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandOK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandKO` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCommandWait` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PreviewCrossCursor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogBackColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCOMMAND` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftSTARTUP` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftALARM` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftCONFIG` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftFEEDBACK` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftPOSITION` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogLeftOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightGOOD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightBAD` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LogRightOTHERS` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property TextBoxColorOverride` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property LinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property VisitedLinkColor` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBorder` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property ControlsBackDisabled` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property DisabledButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `property PressedButtons` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |
| `method ChangeColorBrightness` | DONE | `ui/theme.rs` (`Scheme`, `Theme::of`, `Theme::current`, `change_color_brightness`, `LogRowColor`) |

## `ComWrapper/Emulator.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field emu` | DONE | crates/core/src/com/emulator_link.rs |
| `field opened` | DONE | crates/core/src/com/emulator_link.rs |
| `field closing` | DONE | crates/core/src/com/emulator_link.rs |
| `field buffer` | DONE | crates/core/src/com/emulator_link.rs |
| `ctor Emulator` | DONE | crates/core/src/com/emulator_link.rs |
| `method SendData` | DONE | crates/core/src/com/emulator_link.rs |
| `method Configure` | DONE | crates/core/src/com/emulator_link.rs |
| `method Open` | DONE | crates/core/src/com/emulator_link.rs |
| `method Close` | DONE | crates/core/src/com/emulator_link.rs |
| `method Write` | DONE | crates/core/src/com/emulator_link.rs |
| `method Write` | DONE | crates/core/src/com/emulator_link.rs |
| `method Write` | DONE | crates/core/src/com/emulator_link.rs |
| `method ReadLineBlocking` | DONE | crates/core/src/com/emulator_link.rs |
| `method HasData` | DONE | crates/core/src/com/emulator_link.rs |

## `ComWrapper/IComWrapper.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `enum WrapperType` | DONE | crates/core/src/com/mod.rs |
| `interface IComWrapper` | DONE | crates/core/src/com/mod.rs |

## `ComWrapper/LaserWebESP8266.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field mRemainder` | DONE | crates/core/src/com/websocket.rs |
| `field mAddress` | DONE | crates/core/src/com/websocket.rs |
| `field cln` | DONE | crates/core/src/com/websocket.rs |
| `field buffer` | DONE | crates/core/src/com/websocket.rs |
| `method Configure` | DONE | crates/core/src/com/websocket.rs |
| `method Open` | DONE | crates/core/src/com/websocket.rs |
| `method GetResetDiagnosticString` | DONE | crates/core/src/com/websocket.rs |
| `method Close` | DONE | crates/core/src/com/websocket.rs |
| `method Write` | DONE | crates/core/src/com/websocket.rs |
| `method Write` | DONE | crates/core/src/com/websocket.rs |
| `method Write` | DONE | crates/core/src/com/websocket.rs |
| `method ReadLineBlocking` | DONE | crates/core/src/com/websocket.rs |
| `method HasData` | DONE | crates/core/src/com/websocket.rs |

## `ComWrapper/RJCPSerial.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field com` | DONE | crates/core/src/com/serial.rs |
| `field mPortName` | DONE | crates/core/src/com/serial.rs |
| `field mBaudRate` | DONE | crates/core/src/com/serial.rs |
| `method Configure` | DONE | crates/core/src/com/serial.rs |
| `method Open` | DONE | crates/core/src/com/serial.rs |
| `method GetResetDiagnosticString` | DONE | crates/core/src/com/serial.rs |
| `method Close` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method ReadLineBlocking` | DONE | crates/core/src/com/serial.rs |
| `method HasData` | DONE | crates/core/src/com/serial.rs |

## `ComWrapper/Telnet.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field mAddress` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `field cln` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Configure` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Open` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method GetResetDiagnosticString` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Close` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Write` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Write` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Write` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method ReadLineBlocking` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method HasData` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Parse` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method Parse` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method getPort` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |
| `method getIPfromHost` | DONE | crates/core/src/com/telnet.rs (+ ip_helper.rs) |

## `ComWrapper/UsbSerial.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field com` | DONE | crates/core/src/com/serial.rs |
| `field mPortName` | DONE | crates/core/src/com/serial.rs |
| `field mBaudRate` | DONE | crates/core/src/com/serial.rs |
| `method Configure` | DONE | crates/core/src/com/serial.rs |
| `method Open` | DONE | crates/core/src/com/serial.rs |
| `method GetResetDiagnosticString` | DONE | crates/core/src/com/serial.rs |
| `method Close` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method ReadLineBlocking` | DONE | crates/core/src/com/serial.rs |
| `method HasData` | DONE | crates/core/src/com/serial.rs |
| `method Dispose` | DONE | crates/core/src/com/serial.rs |

## `ComWrapper/UsbSerial2.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field com` | DONE | crates/core/src/com/serial.rs |
| `field mPortName` | DONE | crates/core/src/com/serial.rs |
| `field mBaudRate` | DONE | crates/core/src/com/serial.rs |
| `method Configure` | DONE | crates/core/src/com/serial.rs |
| `method Open` | DONE | crates/core/src/com/serial.rs |
| `method GetResetDiagnosticString` | DONE | crates/core/src/com/serial.rs |
| `method Close` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method Write` | DONE | crates/core/src/com/serial.rs |
| `method ReadLineBlocking` | DONE | crates/core/src/com/serial.rs |
| `method HasData` | DONE | crates/core/src/com/serial.rs |

## `ConnectLogForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ConnectLogForm` | DONE | app/src/forms/main/connect_log.rs |
| `field baudRates` | DONE | app/src/forms/main/connect_log.rs |
| `field currentWrapper` | DONE | app/src/forms/main/connect_log.rs |
| `field mLoadedFileName` | DONE | app/src/forms/main/connect_log.rs |
| `ctor ConnectLogForm` | DONE | app/src/forms/main/connect_log.rs |
| `method SetCore` | DONE | app/src/forms/main/connect_log.rs |
| `method RestoreConf` | DONE | app/src/forms/main/connect_log.rs |
| `method InitSpeedCB` | DONE | app/src/forms/main/connect_log.rs |
| `method InitPortCB` | DONE | app/src/forms/main/connect_log.rs |
| `method TimerUpdate` | DONE | app/src/forms/main/connect_log.rs |
| `method CBPort_SelectedIndexChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method CBPort_TextChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method CBSpeed_SelectedIndexChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method UpdateConf` | DONE | app/src/forms/main/connect_log.rs |
| `method CBProtocol_SelectedIndexChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method TxtHostName_TextChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method ITcpPort_CurrentValueChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method UDLoopCounter_ValueChanged` | DONE | app/src/forms/main/connect_log.rs |
| `method OnColorChange` | DONE | app/src/forms/main/connect_log.rs |
| `method TxtManualCommand_Enter` | DONE | app/src/forms/main/connect_log.rs |
| `method TxtManualCommand_Leave` | DONE | app/src/forms/main/connect_log.rs |
| `method TbFileName_MouseEnter` | DONE | app/src/forms/main/connect_log.rs |
| `method TbFileName_MouseLeave` | DONE | app/src/forms/main/connect_log.rs |
| `method BtnAbortProgram_Click` | DONE | app/src/forms/main/connect_log.rs |
| `method ConfigFromDiscovery` | DONE | app/src/forms/main/connect_log.rs |
| `method ConfigFromWiFiForm` | DONE | app/src/forms/main/connect_log.rs |
| `method CmdLog_Paint` | DONE | app/src/forms/main/connect_log.rs |

## `Core/GrblCore.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `enum Firmware` | DONE | crates/gcode/src/config.rs Firmware |
| `class GrblCore` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field MaterialDB` | N/A | services crate (PSHelper) |
| `field GCODE_STD_HEADER` | DONE | crates/gcode/src/config.rs |
| `field GCODE_STD_PASSES` | DONE | crates/gcode/src/config.rs |
| `field GCODE_STD_FOOTER` | DONE | crates/gcode/src/config.rs |
| `field CH340Version` | N/A | Windows WMI query, not ported (PLAN 6) |
| `class ThreadingMode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field StatusQuery` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field TxLong` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field TxShort` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field RxLong` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field RxShort` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field Name` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor ThreadingMode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ToString` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Equals` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetHashCode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `enum DetectedIssue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `enum MacStatus` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `enum JogDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `enum StreamingMode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class GrblVersionInfo` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `ctor GrblVersionInfo` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `ctor GrblVersionInfo` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `ctor GrblVersionInfo` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method operator !=` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `field operator` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method operator <` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method operator <=` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method operator >` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method operator >=` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method ToString` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method Equals` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method GetHashCode` | N/A | Rust `Hash` not needed (equality only) |
| `method CompareTo` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `method Clone` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property Major` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property Minor` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property IsOrtur` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property IsLonger` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property IsHAL` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property IsLuckyWiFi` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `property MachineName` | DONE | crates/gcode/src/config.rs GrblVersionInfo |
| `delegate void` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `delegate void` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `delegate void` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `delegate void` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `delegate void` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field IssueDetected` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field MachineStatusChanged` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnFileLoading` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnFileLoaded` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnOverrideChange` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnLoopCountChange` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field JogStateChange` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnAutoSizeDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnZoomInDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnZoomOutDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field syncro` | N/A | WinForms sync object; events go over a channel |
| `field com` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field file` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mQueue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mRetryQueue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mPending` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mSent` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mQueuePtr` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mSentPtr` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mWelcomeSeen` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mVendorInfoSeen` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mVendorVersionSeen` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mUsedBuffer` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field DEFAULT_BUFFER_SIZE` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mAutoBufferSize` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mMPos` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mWCO` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mGrblBlocks` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mGrblBuffer` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mFailedConnection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTP` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mMachineStatus` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurF` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurS` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurOvLinear` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurOvRapids` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurOvPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTarOvLinear` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTarOvRapids` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTarOvPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLoopCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field QueryTimer` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field TX` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field RX` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field connectStart` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field debugLastStatusDelay` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field debugLastMoveOrActivityDelay` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mThreadingMode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mHotKeyManager` | N/A | HotKeysManager is not in this package |
| `field UsageCounters` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mDetectedIP` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mDoingSend` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property ShowLaserOffMovements` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property ShowExecutedCommands` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property ShowPerformanceDiagnostic` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property ShowBoundingBox` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property CrossCursor` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property PreviewLineSize` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property AutoSizeOnDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor GrblCore` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor GrblCore` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetCH340Version` | N/A | Windows WMI query, not ported (PLAN 6) |
| `method HotKeyOverride` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ValidateConfig` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetStatus` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsAnyHoldState` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RefreshConfigOnConnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendHomingCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendUnlockCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetIssue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ReOpenFile` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field ImageExtensions` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field GCodeExtensions` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field ProjectFileExtensions` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OpenFile` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetEncoder` | N/A | GDI+ codec, unused |
| `method SaveProgram` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SaveProject` | N/A | project files: services (UI picks path) |
| `method RefreshConfigOnConnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RefreshMachineInfo` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageOptMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageVerMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `enum RefreshCause` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RefreshConfig` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class ReadConfigCountException` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor ReadConfigCountException` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class WriteConfigException` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field ErrorLines` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor WriteConfigException` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property DetectedIP` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method WriteWiFiConfig` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method WriteLongerWiFi` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method WriteOrturWiFi` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method WriteConfig` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RunProgram` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method AbortProgram` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RunProgramFromPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method UserWantToContinue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RunProgramFromStart` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnJobCycle` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnJobBegin` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnJobEnd` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ContinueProgramFromKnown` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnqueueCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Configure` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OpenCom` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CloseCom` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Exiting` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property IsConnected` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CycleStartResume` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method FeedHold` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SafetyDoor` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method QueryPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GrblReset` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method InternalReset` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendBoardResetCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendImmediate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SentCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogToPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogToPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ContinuousJogToPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogToDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogToDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogToDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ContinuousJogToDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JogAbort` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ContinuousJogAbort` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method LimitToBound` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnqueueJogV09` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnqueueJogV09` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnqueueJogV11` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnqueueJogV11` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class ContinuousJog` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Direction` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Speed` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Target` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurrentTargetLock` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mPrev` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurr` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetJogTarget` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetAndClearTarget` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Abort` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ToPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ToDirection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor ContinuousJog` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HandleContinuosJog` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method StartTX` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method InitializeBoard` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ThreadTX` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method DetectHang` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnConnectTimeout` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CanSend` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method BufferIsFull` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method PeekNextCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RemoveManagedCommand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HasSpaceInBuffer` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendLine` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendToSerial` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HandleMissingOK` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method UnlockFromBufferStuck` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsBufferStuck` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CreateFakeOK` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method MachineNotMovingOrReply` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method MachineSayBufferFree` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method BufferIsFree` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HasPendingCommands` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageReceivedLine` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageStaIPMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageLongerIPMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsCommandReplyMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsRealtimeStatusMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsGrblHalWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsVigoWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsOrturModelMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsLongerModelMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsAuferoModelMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsOrturFirmwareMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsLongerFirmwareMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsStandardWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsSimpleLaserWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsUnknownWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsBrokenOkMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsStandardBlockingAlarm` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsIVerMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsIOptMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsStaIPMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsLongerIPMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method IsOrturBlockingAlarm` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageGenericMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageStandardBlockingAlarm` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageOrturBlockingAlarm` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageStandardWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageUnknownWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageSimpleLaserWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageGrblHalWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageVigoWelcomeMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageOrturModelMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageLongerModelMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageLongerFirmwareMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageOrturFirmwareMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnStartupMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method DetectUnexpectedReset` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method StatusReportVersion` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageRealTimeStatus` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ComputeWCO` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseWCO` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseWPos` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseMPos` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseFloat` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseFloat` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseBf` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EnlargeBuffer` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseFS` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseF` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetFS` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetMPosition` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetWCO` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageBrokenOkMessage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageCommandResponse` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field trimarray` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method WaitComLineOrDisconnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HasIncomingData` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageOverrides` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseOverrides` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ChangeOverrides` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ParseMachineStatus` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ForceStatusIdle` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnProgramEnd` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ClearQueue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ClearQueue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property QueueEmpty` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property SupportAutoCooling` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageCoolingCycles` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mHoldByUserRequest` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mHoldByCoolingRequest` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mNowCooling` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method StartCooling` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ResumeCooling` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mDataPath` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTempPath` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method TranslateEnum` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ManageHotKeys` | N/A | hotkeys: services/UI |
| `method GetHotKeyString` | N/A | hotkeys: services/UI |
| `method HKConnectDisconnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HKConnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HKDisconnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HelpOnLine` | N/A | UI (open link) |
| `method GrblHoming` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GrblUnlock` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SetNewZero` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property JogSpeed` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property JogStep` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property ContinuosJogEnabled` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property SuspendHK` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property HotKeys` | N/A | hotkeys: services/UI |
| `method WriteHotkeys` | N/A | hotkeys: services/UI |
| `field bracketsRegEx` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ExecuteCustomCode` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EvaluateExpression` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EvaluateCB` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method FormatNumber` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property CurrentF` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property CurrentS` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method StringToGCode` | N/A | unused private helper |
| `method AutoSizeDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ZoomInDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ZoomOutDrawing` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property UIShowGrblConfig` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property UIShowUnlockButtons` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property IsOrturBoard` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property IsLongerBoard` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property FailedConnectionCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field OnProgramEnded` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class TimeProjection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mETarget` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mEProgress` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mStart` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mEnd` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mGlobalStart` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mGlobalEnd` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mPauseBegin` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCumulatedPause` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mInPause` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCompleted` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mStarted` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTargetCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mExecutedCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mSentCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mErrorCount` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mContinueCorrection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLastKnownWCO` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor TimeProjection` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Reset` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobStart` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobContinue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobSent` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobError` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobExecuted` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobPause` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobResume` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobEnd` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method JobIssue` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class GrblConfST` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field MAX_CONFIG_SPEED` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field MAX_CONFIG_PWM` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field MAX_CONFIG_RESOLUTION` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field MAX_CONFIG_SIZE` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field MAX_CONFIG_ACCEL` | DONE | crates/gcode/src/config.rs GrblConfST |
| `class GrblConfParam` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field mNumber` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field mValue` | DONE | crates/gcode/src/config.rs GrblConfST |
| `ctor GrblConfParam` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method Clone` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field mData` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field mVersion` | DONE | crates/gcode/src/config.rs GrblConfST |
| `ctor GrblConfST` | DONE | crates/gcode/src/config.rs GrblConfST |
| `ctor GrblConfST` | DONE | crates/gcode/src/config.rs GrblConfST |
| `ctor GrblConfST` | DONE | crates/gcode/src/config.rs GrblConfST |
| `ctor GrblConfST` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property GrblVersion` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property NoVersionInfo` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property Version11` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property Version9` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property ExpectedCount` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property HomingEnabled` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property MaxRateX` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property MaxRateY` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property MinPWM` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property MaxPWM` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property ResolutionX` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property ResolutionY` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property TableWidth` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property TableHeight` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property SoftLimit` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property AccelerationXY` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property AccelerationX` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property AccelerationY` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property WiFi_SSID` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property WiFi_Pwd` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property TelnetPort` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method ReadDecimal` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method ReadString` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method ToList` | DONE | crates/gcode/src/config.rs GrblConfST |
| `property Count` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method HasChanges` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method ContainsKey` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method GetEnumerator` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field ConfRegEX` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method IsSetConf` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method AddOrUpdate` | DONE | crates/gcode/src/config.rs GrblConfST |
| `field FoxalienCleanupConfig` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method FixFoxalienConfig` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method SetValueIfKeyExist` | DONE | crates/gcode/src/config.rs GrblConfST |
| `method ValidateConfig` | DONE | crates/gcode/src/config.rs GrblConfST |
| `class GrblConf` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `class GrblConfParam` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `field mNumber` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `field mValue` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `ctor GrblConfParam` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method Clone` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `field mData` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `field mVersion` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `ctor GrblConf` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `ctor GrblConf` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `ctor GrblConf` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property GrblVersion` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property NoVersionInfo` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property Version11` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property Version9` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property ExpectedCount` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property HomingEnabled` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property MaxRateX` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property MaxRateY` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property MinPWM` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property MaxPWM` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property ResolutionX` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property ResolutionY` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property TableWidth` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property TableHeight` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property SoftLimit` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property AccelerationXY` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property AccelerationX` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property AccelerationY` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method ReadWithDefault` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method ToList` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method Add` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `property Count` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method HasChanges` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method ContainsKey` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method SetValue` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method GetEnumerator` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `field ConfRegEX` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method IsSetConf` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method AddOrUpdate` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method SetValueIfKeyExist` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `method ValidateConfig` | DONE | crates/gcode/src/config.rs GrblConf (obsolete, settings migration) |
| `struct GPoint` | DONE | crates/gcode/src/config.rs GPoint |
| `field Z` | DONE | crates/gcode/src/config.rs GPoint |
| `ctor GPoint` | DONE | crates/gcode/src/config.rs GPoint |
| `property Zero` | DONE | crates/gcode/src/config.rs GPoint |
| `field operator` | DONE | crates/gcode/src/config.rs GPoint |
| `method operator !=` | DONE | crates/gcode/src/config.rs GPoint |
| `method operator -` | DONE | crates/gcode/src/config.rs GPoint |
| `method operator +` | DONE | crates/gcode/src/config.rs GPoint |
| `method Equals` | DONE | crates/gcode/src/config.rs GPoint |
| `method GetHashCode` | N/A | Rust `Hash` not needed (equality only) |
| `method ToPointF` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class LaserLifeHandler` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class ListLLC` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field LastAttempt` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field LastSent` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetClone` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLLCL` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mCurrentLLC` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLastStatusHiResTimeNano` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLastPowerHiResTimeNano` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLastSave` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLLCFileName` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class LaserLifeCounter` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field DEF_NAME` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field DEF_BRAND` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field DEF_MODEL` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mGuid` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mName` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mBrand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mModuleModel` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mOpticalPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mPurchaseDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mMonitoringDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mDeathDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mLastUsage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTimeInRun` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTimeUsageNormalizedPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTimeUsageNonZero` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `field mTimeClasses` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property PurchaseDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property MonitoringDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property DeathDate` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property LastUsage` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Name` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Brand` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Model` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property OpticalPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property TimeInRun` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property TimeUsageNormalizedPower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property StressTime` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property AveragePowerFactor` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Guid` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Classes` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor LaserLifeCounter` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CreateNew` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method CreateDefault` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method AddRunTime` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Clone` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method AddTrueLaserTimePower` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Update` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method HasWorked` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `ctor LaserLifeHandler` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ComputeLaserTime` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SendAndSave` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method ComputeLaserTrueTime` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnConnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method OnDisconnect` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method DoSave` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method DoSend` | N/A | telemetry, PLAN 6 (never sends) |
| `method GetListClone` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method SaveNow` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RealDoSend` | N/A | telemetry, PLAN 6 (never sends) |
| `method BuildJson` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method EscapeJson` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `class RealDoSendRV` | N/A | telemetry, PLAN 6 |
| `field UpdateResult` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `property Success` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method RealDoSave` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method GetCurrentTime` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Add` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Edit` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Delete` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method Death` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |
| `method UnDeath` | DONE | crates/core/src/grbl_core/ (mod.rs, state.rs, types.rs, jog.rs, expr.rs, time_projection.rs), laser_life.rs |

## `Core/MarlinCore.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class MarlinCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `ctor MarlinCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method QueryPosition` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `property CurrentStreamingMode` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `property UIShowGrblConfig` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `property UIShowUnlockButtons` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `property SupportTrueJogging` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method SendUnlockCommand` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method SendBoardResetCommand` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method ParseMachineStatus` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method RefreshConfig` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method RefreshMachineInfo` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method DetectHang` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method ManageReceivedLine` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method IsMarlinRealTimeStatusMessage` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method ManageMarlinRealTimeStatus` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |
| `method ForceStatusIdle` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Marlin match arms) |

## `Core/SmoothieCore.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class SmoothieCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `ctor SmoothieCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `method InitializeBoard` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `method ParseF` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `property CurrentStreamingMode` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `property UIShowGrblConfig` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `property UIShowUnlockButtons` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `property SupportTrueJogging` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `method SendUnlockCommand` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |
| `method SendBoardResetCommand` | DONE | crates/core/src/grbl_core/state.rs (Firmware::Smoothie match arms) |

## `Core/VigoCore.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class VigoCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `ctor VigoCore` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method QueryPosition` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `property BufferSize` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method ManageReceivedLine` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method IsVigoStatusMessage` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method ManageVigoStatus` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method ParseVSta` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `field injob` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `field mOldReceived` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `field mOldManaged` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `field mOldError` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method ParseSBuf` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method OnJobBegin` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method OnJobEnd` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |
| `method SendToSerial` | DONE | crates/core/src/grbl_core/state.rs (Firmware::VigoWork match arms) |

## `CsPotrace/BezierToBiarc/Algorithm.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `class Algorithm` | DONE | crates/potrace/src/biarc/algorithm.rs |
| `method IsRealInflexionPoint` | DONE | crates/potrace/src/biarc/algorithm.rs |
| `method ApproxCubicBezier` | DONE | crates/potrace/src/biarc/algorithm.rs |

## `CsPotrace/BezierToBiarc/Arc.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct Arc` | DONE | crates/potrace/src/biarc/arc.rs |
| `field C` | DONE | crates/potrace/src/biarc/arc.rs |
| `field r` | DONE | crates/potrace/src/biarc/arc.rs |
| `field startAngle` | DONE | crates/potrace/src/biarc/arc.rs |
| `field sweepAngle` | DONE | crates/potrace/src/biarc/arc.rs |
| `field P1` | DONE | crates/potrace/src/biarc/arc.rs |
| `field P2` | DONE | crates/potrace/src/biarc/arc.rs |
| `ctor Arc` | DONE | crates/potrace/src/biarc/arc.rs |
| `method PointAt` | DONE | crates/potrace/src/biarc/arc.rs |

## `CsPotrace/BezierToBiarc/BiArc.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct BiArc` | DONE | crates/potrace/src/biarc/arc.rs |
| `field A1` | DONE | crates/potrace/src/biarc/arc.rs |
| `field A2` | DONE | crates/potrace/src/biarc/arc.rs |
| `ctor BiArc` | DONE | crates/potrace/src/biarc/arc.rs |
| `method PointAt` | DONE | crates/potrace/src/biarc/arc.rs |

## `CsPotrace/BezierToBiarc/Complex.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct Complex` | DONE | crates/potrace/src/biarc/complex.rs |
| `field Zero` | DONE | crates/potrace/src/biarc/complex.rs |
| `field One` | DONE | crates/potrace/src/biarc/complex.rs |
| `field ImaginaryOne` | DONE | crates/potrace/src/biarc/complex.rs |
| `field m_real` | DONE | crates/potrace/src/biarc/complex.rs |
| `field m_imaginary` | DONE | crates/potrace/src/biarc/complex.rs |
| `field LOG_10_INV` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `ctor Complex` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method Complex` | N/A | implicit numeric conversions not needed (Complex::from_real) |
| `method operator -` | DONE | crates/potrace/src/biarc/complex.rs |
| `method operator +` | DONE | crates/potrace/src/biarc/complex.rs |
| `method operator -` | DONE | crates/potrace/src/biarc/complex.rs |
| `method operator *` | DONE | crates/potrace/src/biarc/complex.rs |
| `method operator /` | DONE | crates/potrace/src/biarc/complex.rs |
| `field operator` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method operator !=` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method FromPolarCoordinates` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Negate` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Add` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Subtract` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Multiply` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Divide` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Abs` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Conjugate` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Reciprocal` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Equals` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Equals` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method ToString` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method ToString` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method ToString` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method ToString` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method GetHashCode` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Sin` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Sinh` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Asin` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Cos` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Cosh` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Acos` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Tan` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Tanh` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Atan` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Log` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Log` | N/A | unused overload |
| `method Log10` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Exp` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Sqrt` | DONE | crates/potrace/src/biarc/complex.rs |
| `method Pow` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Pow` | N/A | unused by the app / covered by Rust traits and PartialEq |
| `method Scale` | N/A | unused by the app / covered by Rust traits and PartialEq |

## `CsPotrace/BezierToBiarc/CubicBezier.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct CubicBezier` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `field P1` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `field P2` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `field C1` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `field C2` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `ctor CubicBezier` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `method PointAt` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |
| `method Split` | DONE | crates/potrace/src/biarc/cubic_bezier.rs |

## `CsPotrace/BezierToBiarc/Line.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct Line` | DONE | crates/potrace/src/biarc/line.rs |
| `field m` | DONE | crates/potrace/src/biarc/line.rs |
| `field P` | DONE | crates/potrace/src/biarc/line.rs |
| `ctor Line` | DONE | crates/potrace/src/biarc/line.rs |
| `ctor Line` | DONE | crates/potrace/src/biarc/line.rs |
| `method Intersection` | DONE | crates/potrace/src/biarc/line.rs |
| `method VerticalIntersection` | DONE | crates/potrace/src/biarc/line.rs |
| `method CreatePerpendicularAt` | DONE | crates/potrace/src/biarc/line.rs |
| `method Slope` | DONE | crates/potrace/src/biarc/line.rs |

## `CsPotrace/BezierToBiarc/Tuple.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `class Tuple` | N/A | replaced by Rust tuples |
| `field m_Item1` | N/A | replaced by Rust tuples |
| `field m_Item2` | N/A | replaced by Rust tuples |
| `property Item1` | N/A | replaced by Rust tuples |
| `property Item2` | N/A | replaced by Rust tuples |
| `ctor Tuple` | N/A | replaced by Rust tuples |

## `CsPotrace/BezierToBiarc/Vector2.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `struct Vector2` | DONE | crates/potrace/src/biarc/vector2.rs |
| `field X` | DONE | crates/potrace/src/biarc/vector2.rs |
| `field Y` | DONE | crates/potrace/src/biarc/vector2.rs |
| `ctor Vector2` | DONE | crates/potrace/src/biarc/vector2.rs |
| `ctor Vector2` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator +` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator -` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator *` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator *` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator *` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator /` | N/A | Vector2/Vector2 unused |
| `method operator /` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method operator -` | DONE | crates/potrace/src/biarc/vector2.rs |
| `field operator` | N/A | unused by the app / Rust derive |
| `method operator !=` | N/A | unused by the app / Rust derive |
| `method GetHashCode` | N/A | unused by the app / Rust derive |
| `method Equals` | N/A | unused by the app / Rust derive |
| `method ToString` | N/A | unused by the app / Rust derive |
| `method ToString` | N/A | unused by the app / Rust derive |
| `method ToString` | N/A | unused by the app / Rust derive |
| `method Length` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method LengthSquared` | N/A | unused by the app / Rust derive |
| `method Distance` | DONE | crates/potrace/src/biarc/vector2.rs |
| `method DistanceSquared` | N/A | unused by the app / Rust derive |
| `method Normalize` | N/A | unused by the app / Rust derive |
| `method Reflect` | N/A | unused by the app / Rust derive |
| `method Clamp` | N/A | unused by the app / Rust derive |
| `method Lerp` | N/A | unused by the app / Rust derive |
| `method Add` | N/A | unused by the app / Rust derive |
| `method Subtract` | N/A | unused by the app / Rust derive |
| `method Multiply` | N/A | unused by the app / Rust derive |
| `method Multiply` | N/A | unused by the app / Rust derive |
| `method Multiply` | N/A | unused by the app / Rust derive |
| `method Divide` | N/A | unused by the app / Rust derive |
| `method Divide` | N/A | unused by the app / Rust derive |
| `method Negate` | N/A | unused by the app / Rust derive |
| `method CopyTo` | N/A | unused by the app / Rust derive |
| `method CopyTo` | N/A | unused by the app / Rust derive |
| `method Equals` | N/A | unused by the app / Rust derive |
| `method Dot` | N/A | unused by the app / Rust derive |
| `method Min` | N/A | unused by the app / Rust derive |
| `method Max` | N/A | unused by the app / Rust derive |
| `method Abs` | N/A | unused by the app / Rust derive |
| `method SquareRoot` | N/A | unused by the app / Rust derive |

## `CsPotrace/CsPotrace.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `enum CurveKind` | DONE | crates/potrace/src/trace.rs |
| `class dPoint` | DONE | crates/potrace/src/trace.rs |
| `field X` | DONE | crates/potrace/src/trace.rs |
| `field Y` | DONE | crates/potrace/src/trace.rs |
| `ctor dPoint` | DONE | crates/potrace/src/trace.rs |
| `method copy` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `ctor dPoint` | DONE | crates/potrace/src/trace.rs |
| `struct Curve` | DONE | crates/potrace/src/trace.rs |
| `field Kind` | DONE | crates/potrace/src/trace.rs |
| `field A` | DONE | crates/potrace/src/trace.rs |
| `field ControlPointA` | DONE | crates/potrace/src/trace.rs |
| `field ControlPointB` | DONE | crates/potrace/src/trace.rs |
| `field B` | DONE | crates/potrace/src/trace.rs |
| `ctor Curve` | DONE | crates/potrace/src/trace.rs |
| `enum TurnPolicy` | DONE | crates/potrace/src/trace.rs |
| `class Potrace` | DONE | crates/potrace/src/trace.rs |
| `field turnpolicy` | DONE | crates/potrace/src/trace.rs |
| `field POTRACE_CORNER` | DONE | crates/potrace/src/trace.rs |
| `field POTRACE_CURVETO` | DONE | crates/potrace/src/trace.rs |
| `field COS179` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `field turdsize` | DONE | crates/potrace/src/trace.rs |
| `field alphamax` | DONE | crates/potrace/src/trace.rs |
| `field curveoptimizing` | DONE | crates/potrace/src/trace.rs |
| `field opttolerance` | DONE | crates/potrace/src/trace.rs |
| `field Treshold` | DONE | crates/potrace/src/trace.rs |
| `ctor Point` | DONE | crates/potrace/src/trace.rs |
| `field x` | DONE | crates/potrace/src/trace.rs |
| `field y` | DONE | crates/potrace/src/trace.rs |
| `ctor Bitmap_p` | DONE | crates/potrace/src/trace.rs |
| `field w` | DONE | crates/potrace/src/trace.rs |
| `field h` | DONE | crates/potrace/src/trace.rs |
| `method at` | DONE | crates/potrace/src/trace.rs |
| `field data` | DONE | crates/potrace/src/trace.rs |
| `method index` | DONE | crates/potrace/src/trace.rs |
| `method flip` | DONE | crates/potrace/src/trace.rs |
| `method copy` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `field m` | DONE | crates/potrace/src/trace.rs |
| `field area` | DONE | crates/potrace/src/trace.rs |
| `field len` | DONE | crates/potrace/src/trace.rs |
| `field sign` | DONE | crates/potrace/src/trace.rs |
| `field pt` | DONE | crates/potrace/src/trace.rs |
| `field minX` | DONE | crates/potrace/src/trace.rs |
| `field minY` | DONE | crates/potrace/src/trace.rs |
| `field maxX` | DONE | crates/potrace/src/trace.rs |
| `field maxY` | DONE | crates/potrace/src/trace.rs |
| `field x0` | DONE | crates/potrace/src/trace.rs |
| `field y0` | DONE | crates/potrace/src/trace.rs |
| `field po` | DONE | crates/potrace/src/trace.rs |
| `field lon` | DONE | crates/potrace/src/trace.rs |
| `field sums` | DONE | crates/potrace/src/trace.rs |
| `field curve` | DONE | crates/potrace/src/trace.rs |
| `field n` | DONE | crates/potrace/src/trace.rs |
| `field tag` | DONE | crates/potrace/src/trace.rs |
| `field vertex` | DONE | crates/potrace/src/trace.rs |
| `field c` | DONE | crates/potrace/src/trace.rs |
| `field alpha` | DONE | crates/potrace/src/trace.rs |
| `field alpha0` | DONE | crates/potrace/src/trace.rs |
| `field beta` | DONE | crates/potrace/src/trace.rs |
| `field alphacurve` | DONE | crates/potrace/src/trace.rs |
| `ctor privcurve` | DONE | crates/potrace/src/trace.rs |
| `method interval` | DONE | crates/potrace/src/trace.rs |
| `method dorth_infty` | DONE | crates/potrace/src/trace.rs |
| `method ddenom` | DONE | crates/potrace/src/trace.rs |
| `method dpara` | DONE | crates/potrace/src/trace.rs |
| `method cprod` | DONE | crates/potrace/src/trace.rs |
| `method iprod` | DONE | crates/potrace/src/trace.rs |
| `method iprod1` | DONE | crates/potrace/src/trace.rs |
| `method ddist` | DONE | crates/potrace/src/trace.rs |
| `ctor Sum` | DONE | crates/potrace/src/trace.rs |
| `field y2` | DONE | crates/potrace/src/trace.rs |
| `method xprod` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method xprodi` | DONE | crates/potrace/src/trace.rs |
| `method cyclic` | DONE | crates/potrace/src/trace.rs |
| `method sign` | DONE | crates/potrace/src/trace.rs |
| `method quadform` | DONE | crates/potrace/src/trace.rs |
| `method bezier` | DONE | crates/potrace/src/trace.rs |
| `method tangent` | DONE | crates/potrace/src/trace.rs |
| `method pointslope` | DONE | crates/potrace/src/trace.rs |
| `method abs` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method min` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method max` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method sq` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method cu` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method mod` | DONE | crates/potrace/src/trace.rs |
| `method floordiv` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |
| `method ConvertBitmap` | DONE | crates/potrace/src/trace.rs |
| `method findNext` | DONE | crates/potrace/src/trace.rs |
| `method findPath` | DONE | crates/potrace/src/trace.rs |
| `method bmToPathlist` | DONE | crates/potrace/src/trace.rs |
| `method xorPath` | DONE | crates/potrace/src/trace.rs |
| `method calcSums` | DONE | crates/potrace/src/trace.rs |
| `method penalty3` | DONE | crates/potrace/src/trace.rs |
| `method calcLon` | DONE | crates/potrace/src/trace.rs |
| `method bestPolygon` | DONE | crates/potrace/src/trace.rs |
| `method adjustVertices` | DONE | crates/potrace/src/trace.rs |
| `method reverse` | DONE | crates/potrace/src/trace.rs |
| `method smooth` | DONE | crates/potrace/src/trace.rs |
| `field pen` | DONE | crates/potrace/src/trace.rs |
| `field c` | DONE | crates/potrace/src/trace.rs |
| `field t` | DONE | crates/potrace/src/trace.rs |
| `field s` | DONE | crates/potrace/src/trace.rs |
| `field alpha` | DONE | crates/potrace/src/trace.rs |
| `method opti_penalty` | DONE | crates/potrace/src/trace.rs |
| `method optiCurve` | DONE | crates/potrace/src/trace.rs |
| `ctor Quad` | DONE | crates/potrace/src/trace.rs |
| `method at` | DONE | crates/potrace/src/trace.rs |
| `field data` | DONE | crates/potrace/src/trace.rs |
| `field bm` | DONE | crates/potrace/src/trace.rs |
| `method majority` | DONE | crates/potrace/src/trace.rs |
| `field pathlist` | DONE | crates/potrace/src/trace.rs |
| `method tracetoList` | DONE | crates/potrace/src/trace.rs |
| `method Clear` | DONE | crates/potrace/src/trace.rs |
| `method PotraceTrace` | DONE | crates/potrace/src/trace.rs |
| `method toString` | DONE | crates/potrace/src/trace.rs |
| `method segment` | DONE | crates/potrace/src/trace.rs |
| `method bezier` | DONE | crates/potrace/src/trace.rs |
| `method path` | DONE | crates/potrace/src/trace.rs |
| `method getSVG` | DONE | crates/potrace/src/trace.rs |

## `CsPotrace/CsPotraceExport.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `class Potrace` | DONE | crates/potrace/src/export.rs |
| `method Export2GDIPlus` | DONE | crates/potrace/src/export.rs |

## `CsPotrace/CsPotraceExportGCODE.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `class Potrace` | DONE | crates/potrace/src/export_gcode.rs |
| `method Export2GCode` | DONE | crates/potrace/src/export_gcode.rs |
| `method GetPathGC` | DONE | crates/potrace/src/export_gcode.rs |
| `method OnPathSegment` | DONE | crates/potrace/src/export_gcode.rs |
| `method OnPathBegin` | DONE | crates/potrace/src/export_gcode.rs |
| `method OnPathEnd` | DONE | crates/potrace/src/export_gcode.rs |
| `method GetArcGC` | DONE | crates/potrace/src/export_gcode.rs |
| `method CasoLimite` | DONE | crates/potrace/src/export_gcode.rs |
| `method formatnumber` | DONE | crates/potrace/src/export_gcode.rs |
| `method AsPointF` | N/A | unused in the app or replaced by std/derive (Clone, Copy, abs, min, max) |

## `CsPotrace/PotraceClipper.cs` (WP-B2)

| member | status | Rust location / reason |
|---|---|---|
| `field resolution` | DONE | crates/potrace/src/clipper.rs |
| `method BuildFilling` | DONE | crates/potrace/src/clipper.rs |
| `method BuildGridFilling` | DONE | crates/potrace/src/clipper.rs |
| `method BuildInsetFilling` | DONE | crates/potrace/src/clipper.rs |
| `method ToPotraceList` | DONE | crates/potrace/src/clipper.rs |
| `method AddGridClip` | DONE | crates/potrace/src/clipper.rs |
| `method AddOffsetSubject` | DONE | crates/potrace/src/clipper.rs |
| `method AddClip` | DONE | crates/potrace/src/clipper.rs |
| `method AddClip` | DONE | crates/potrace/src/clipper.rs |
| `method AddGridSubject` | DONE | crates/potrace/src/clipper.rs |
| `method AddSegment` | DONE | crates/potrace/src/clipper.rs |
| `method AddPathPoint` | DONE | crates/potrace/src/clipper.rs |
| `method AddPathPoints` | DONE | crates/potrace/src/clipper.rs |
| `class Hilbert` | DONE | crates/potrace/src/biarc_free.rs |
| `ctor Hilbert` | DONE | crates/potrace/src/biarc_free.rs |
| `method Execute` | DONE | crates/potrace/src/biarc_free.rs |
| `method DoHilbert` | DONE | crates/potrace/src/biarc_free.rs |

## `CustomButton.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class CustomButtons` | DONE | crates/services/src/custom_button.rs |
| `field buttons` | DONE | crates/services/src/custom_button.rs |
| `property UserFile` | DONE | crates/services/src/custom_button.rs |
| `property StandardFile` | DONE | crates/services/src/custom_button.rs |
| `method LoadFile` | DONE | crates/services/src/custom_button.rs |
| `method SaveFile` | DONE | crates/services/src/custom_button.rs |
| `method Reorder` | DONE | crates/services/src/custom_button.rs |
| `method Add` | DONE | crates/services/src/custom_button.rs |
| `method Remove` | DONE | crates/services/src/custom_button.rs |
| `method GetButton` | DONE | crates/services/src/custom_button.rs |
| `method Export` | DONE | crates/services/src/custom_button.rs |
| `method Import` | DONE | crates/services/src/custom_button.rs |
| `property Count` | DONE | crates/services/src/custom_button.rs |
| `class CustomButton` | DONE | crates/services/src/custom_button.rs (UI: app/src/forms/preview_form/custom_button_ib.rs) |
| `enum EnableStyles` | DONE | crates/services/src/custom_button.rs |
| `enum ButtonTypes` | DONE | crates/services/src/custom_button.rs |
| `field guid` | DONE | crates/services/src/custom_button.rs |
| `field Image` | DONE | crates/services/src/custom_button.rs |
| `field GCode` | DONE | crates/services/src/custom_button.rs |
| `field GCode2` | DONE | crates/services/src/custom_button.rs |
| `field Caption` | DONE | crates/services/src/custom_button.rs |
| `field ToolTip` | DONE | crates/services/src/custom_button.rs |
| `field EnableStyle` | DONE | crates/services/src/custom_button.rs |
| `field ButtonType` | DONE | crates/services/src/custom_button.rs |
| `method EnabledNow` | DONE | crates/services/src/custom_button.rs |

## `CustomButtonForm.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class CustomButtonForm` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `ctor CustomButtonForm` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method LblCaption_Click` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method ShowDialog` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method OnLoad` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method BtnCreate_Click` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method BTOpenImage_Click` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method CbByttonType_SelectedIndexChanged` | DONE | app/src/forms/preview_form/custom_button_form.rs |
| `method LblDescription_LinkClicked` | DONE | app/src/forms/preview_form/custom_button_form.rs |

## `ExceptionManager.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ExceptionManager` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `field Core` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `field ParentMain` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `field UserClose` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `ctor ExceptionManager` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method RegisterHandler` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method OnUnhandledMainException` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method OnUnhandledThreadException` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method OnHandledException` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `delegate void` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method CreateAndShow` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method BtnAbort_Click` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method BtnContinue_Click` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method AppendExceptionData` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method LblFormDescription_LinkClicked` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method ExceptionManager_Load` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method BtnClipboardCopy_Click` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |
| `method TimLink_Tick` | DONE | app/src/forms/main/exception.rs (panic hook + dialog) |

## `Firmware/FlashGrbl.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class FlashGrbl` | DONE | app/src/forms/dialogs/flash_grbl.rs |
| `field SELECT` | DONE | crates/services/src/flash_grbl.rs SELECT_CUSTOM |
| `field retval` | DONE | crates/services/src/flash_grbl.rs run_avrdude return value |
| `field baudRates` | DONE | crates/services/src/flash_grbl.rs BAUD_RATES |
| `ctor FlashGrbl` | DONE | app/src/forms/dialogs/flash_grbl.rs |
| `method InitCbBaudRate` | DONE | crates/services/src/flash_grbl.rs BAUD_RATES/DEFAULT_BAUD |
| `method InitCBFirmware` | DONE | crates/services/src/flash_grbl.rs list_firmwares |
| `method InitPortCB` | DONE | crates/services/src/flash_grbl.rs purge_port_name |
| `class Firmware` | DONE | crates/services/src/flash_grbl.rs Firmware |
| `field path` | DONE | crates/services/src/flash_grbl.rs Firmware.path |
| `ctor Firmware` | DONE | crates/services/src/flash_grbl.rs Firmware::new |
| `method ToString` | DONE | crates/services/src/flash_grbl.rs Firmware::display_name |
| `method BtnOK_Click` | DONE | crates/services/src/flash_grbl.rs avrdude_args/run_avrdude (dialogs are UI) |
| `method CbTarget_SelectedIndexChanged` | DONE | crates/services/src/flash_grbl.rs baud_for_target |
| `method BtnTarget_Click` | DONE | app/src/forms/dialogs/flash_grbl.rs |
| `method BtnFirmware_Click` | DONE | app/src/forms/dialogs/flash_grbl.rs |
| `method CbFirmware_SelectedIndexChanged` | DONE | app/src/forms/dialogs/flash_grbl.rs |
| `method AddOrSelect` | DONE | crates/services/src/flash_grbl.rs add_or_select |
| `method UpdateBtnOK` | DONE | crates/services/src/flash_grbl.rs can_flash |
| `method CbPort_TextChanged` | DONE | app/src/forms/dialogs/flash_grbl.rs |

## `Generator/CuttingTest.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class CuttingTest` | DONE | app/src/forms/generators/cutting_test.rs |
| `field LaserOptions` | DONE | app/src/forms/generators/cutting_test.rs |
| `class ComboboxItem` | DONE | app/src/forms/generators/cutting_test.rs |
| `property Text` | DONE | app/src/forms/generators/cutting_test.rs |
| `property Value` | DONE | app/src/forms/generators/cutting_test.rs |
| `ctor ComboboxItem` | DONE | app/src/forms/generators/cutting_test.rs |
| `method ToString` | DONE | app/src/forms/generators/cutting_test.rs |
| `ctor CuttingTest` | DONE | app/src/forms/generators/cutting_test.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/generators/cutting_test.rs |
| `method GreyScaleForm_Load` | DONE | app/src/forms/generators/cutting_test.rs |
| `method RestoreDefault` | DONE | app/src/forms/generators/cutting_test.rs |
| `method AssignMinMaxLimit` | DONE | app/src/forms/generators/cutting_test.rs |
| `method BtnCreate_Click` | DONE | app/src/forms/generators/cutting_test.rs |
| `method Ii_PCurrentValueChanged` | DONE | app/src/forms/generators/cutting_test.rs |
| `method IiF_CurrentValueChanged` | DONE | app/src/forms/generators/cutting_test.rs |
| `method ComputeSize` | DONE | app/src/forms/generators/cutting_test.rs |
| `method BtnOnOffInfo_Click` | DONE | app/src/forms/generators/cutting_test.rs |

## `Generator/PowerVsSpeedForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class PowerVsSpeedForm` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `field LaserOptions` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `class ComboboxItem` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `property Text` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `property Value` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `ctor ComboboxItem` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method ToString` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `ctor PowerVsSpeedForm` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method GreyScaleForm_Load` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method RestoreDefault` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method AssignMinMaxLimit` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method BtnCreate_Click` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method IiS_CurrentValueChanged` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method IiF_CurrentValueChanged` | DONE | app/src/forms/generators/power_vs_speed.rs |
| `method BtnOnOffInfo_Click` | DONE | app/src/forms/generators/power_vs_speed.rs |

## `Generator/ShakeTest.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class ShakeTest` | DONE | app/src/forms/generators/shake_test.rs |
| `ctor ShakeTest` | DONE | app/src/forms/generators/shake_test.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/generators/shake_test.rs |
| `method BtnCreate_Click` | DONE | app/src/forms/generators/shake_test.rs |
| `method ShakeTest_Load` | DONE | app/src/forms/generators/shake_test.rs |
| `method RestoreDefault` | DONE | app/src/forms/generators/shake_test.rs |
| `method AssignMinMaxLimit` | DONE | app/src/forms/generators/shake_test.rs |
| `method CbAxis_SelectedIndexChanged` | DONE | app/src/forms/generators/shake_test.rs |

## `GrblCommand.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `interface IGrblRow` | DONE | crates/gcode/src/command.rs |
| `class CSVD` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `field Settings` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `field Alarms` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `field Errors` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `method LoadAppropriateCSV` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `method LoadAppropriateSettings` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `method LoadAppropriateAlarms` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `method LoadAppropriateErrors` | DONE | crates/gcode/src/csvd.rs (+ crates/base/src/csv/csvd.rs) |
| `class GrblCommand` | DONE | crates/gcode/src/command.rs |
| `class Element` | DONE | crates/gcode/src/command.rs |
| `field mCommand` | DONE | crates/gcode/src/command.rs |
| `field mNumber` | DONE | crates/gcode/src/command.rs |
| `method Element` | DONE | crates/gcode/src/command.rs |
| `ctor Element` | DONE | crates/gcode/src/command.rs |
| `method ToString` | DONE | crates/gcode/src/command.rs |
| `method Equals` | DONE | crates/gcode/src/command.rs |
| `method GetHashCode` | DONE | crates/gcode/src/command.rs |
| `class ResultWrapper` | DONE | crates/gcode/src/command.rs |
| `field CodedResult` | DONE | crates/gcode/src/command.rs |
| `field mLine` | DONE | crates/gcode/src/command.rs |
| `field mResultWrapper` | DONE | crates/gcode/src/command.rs |
| `field mTimeOffset` | DONE | crates/gcode/src/command.rs |
| `field mHelper` | DONE | crates/gcode/src/command.rs |
| `field mRepeatCount` | DONE | crates/gcode/src/command.rs |
| `property LinkedDisplayList` | DONE | crates/gcode/src/command.rs |
| `ctor GrblCommand` | DONE | crates/gcode/src/command.rs |
| `method ClearResult` | DONE | crates/gcode/src/command.rs |
| `ctor GrblCommand` | DONE | crates/gcode/src/command.rs |
| `ctor GrblCommand` | DONE | crates/gcode/src/command.rs |
| `method BuildHelper` | DONE | crates/gcode/src/command.rs |
| `method DeleteHelper` | DONE | crates/gcode/src/command.rs |
| `method SetOffset` | DONE | crates/gcode/src/command.rs |
| `enum CommandStatus` | DONE | crates/gcode/src/command.rs |
| `method Clone` | DONE | crates/gcode/src/command.rs |
| `field trimarray` | DONE | crates/gcode/src/command.rs |
| `method GetResult` | DONE | crates/gcode/src/command.rs |
| `method Add` | DONE | crates/gcode/src/command.rs |
| `method SetResult` | DONE | crates/gcode/src/command.rs |
| `method IsCW` | DONE | crates/gcode/src/command.rs |
| `method GetElement` | DONE | crates/gcode/src/command.rs |
| `method GetDecodedMessage` | DONE | crates/gcode/src/command.rs |
| `method GetToolTip` | DONE | crates/gcode/src/command.rs |
| `method SetSending` | DONE | crates/gcode/src/command.rs |
| `method ToString` | DONE | crates/gcode/src/command.rs |
| `method Dispose` | DONE | crates/gcode/src/command.rs |
| `class GrblMessage` | DONE | crates/gcode/src/command.rs |
| `enum MessageType` | DONE | crates/gcode/src/command.rs |
| `field mNativeMessage` | DONE | crates/gcode/src/command.rs |
| `field mMessage` | DONE | crates/gcode/src/command.rs |
| `field mToolTip` | DONE | crates/gcode/src/command.rs |
| `field mType` | DONE | crates/gcode/src/command.rs |
| `ctor GrblMessage` | DONE | crates/gcode/src/command.rs |
| `ctor GrblMessage` | DONE | crates/gcode/src/command.rs |
| `method GetNativeMessage` | DONE | crates/gcode/src/command.rs |
| `method GetDecodedMessage` | DONE | crates/gcode/src/command.rs |
| `method GetResult` | DONE | crates/gcode/src/command.rs |
| `method GetToolTip` | DONE | crates/gcode/src/command.rs |

## `GrblConfig.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblConfig` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `field Core` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `field mLocalCopy` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `ctor GrblConfig` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method Dispose` | N/A | WinForms designer / window-style boilerplate |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method BtnRead_Click` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method DoRead` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method BtnWrite_Click` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method WriteConf` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method BtnExport_Click` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method BtnImport_Click` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method DGV_DataError` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method ActionTimer_Tick` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method ActionResult` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method GrblConfig_FormClosing` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method GetChanges` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method HasChanges` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method GrblConfig_Load` | DONE | app/src/forms/dialogs/grbl_config.rs |
| `method DGV_CellValidating` | DONE | app/src/forms/dialogs/grbl_config.rs |

## `GrblEmulator/EmulatorUI.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class EmulatorUI` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field istance` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field canclose` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field CP_NOCLOSE_BUTTON` | N/A | Win32 class style hiding the close box; the egui window (emulator_ui.rs) is created without a close box, same behaviour |
| `method ShowUI` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method HideUI` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `ctor EmulatorUI` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method RT_Tick` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method EmulatorUI_FormClosing` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `class RollingBuffer` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field buffer` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field head` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field tail` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field count` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `field capacity` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `ctor RollingBuffer` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method Add` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method ToArray` | DONE | app/src/forms/dialogs/emulator_ui.rs |
| `method Clear` | DONE | app/src/forms/dialogs/emulator_ui.rs |

## `GrblEmulator/Grblv11Emulator.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class Grblv11Emulator` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field EMULATE_RANDOM_ERRORS` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field EMULATE_MISSING_OK` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field filename` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field EmulatorMessage` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field mPaused` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field mCheck` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field SPB` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field RX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field TX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field rxBuf` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field txBuf` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `delegate void` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field mSendFunc` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field opened` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field conf` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `field rng` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `ctor Grblv11Emulator` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method CloseCom` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method EmuLog` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method OpenCom` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method ManageMessage` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method Pause` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method IsSetConf` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SetConfig` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SendConfig` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method EnqueueRX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method GrblReset` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SendVersion` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SendStatus` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method ManageRX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SendInfo` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method EmulateHoming` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method SwapCheck` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method ManageTX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method EnqueueTX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method ImmediateTX` | DONE | crates/core/src/emulator/grbl_v11.rs |
| `method EmulateCommand` | DONE | crates/core/src/emulator/grbl_v11.rs |

## `GrblEmulator/WebsocketEmulator.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `field srv` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method Start` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method Stop` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `class GrblWebSocketEmulator` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `field emu` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `ctor GrblWebSocketEmulator` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method SendData` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method OnClose` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method OnOpen` | DONE | crates/core/src/emulator/websocket_emulator.rs |
| `method OnMessage` | DONE | crates/core/src/emulator/websocket_emulator.rs |

## `GrblFile.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblFile` | DONE | crates/gcode/src/grbl_file.rs |
| `enum CartesianQuadrant` | DONE | crates/gcode/src/grbl_file.rs |
| `delegate void` | DONE | crates/gcode/src/grbl_file.rs |
| `field OnFileLoading` | DONE | crates/gcode/src/grbl_file.rs |
| `field OnFileLoaded` | DONE | crates/gcode/src/grbl_file.rs |
| `field list` | DONE | crates/gcode/src/grbl_file.rs |
| `field mRange` | DONE | crates/gcode/src/grbl_file.rs |
| `field mEstimatedTotalTime` | DONE | crates/gcode/src/grbl_file.rs |
| `property Commands` | DONE | crates/gcode/src/grbl_file.rs |
| `field mLoadingThread` | DONE | crates/gcode/src/grbl_file.rs |
| `ctor GrblFile` | DONE | crates/gcode/src/grbl_file.rs |
| `ctor GrblFile` | DONE | crates/gcode/src/grbl_file.rs |
| `method ClearList` | DONE | crates/gcode/src/grbl_file.rs |
| `method SaveGCODE` | DONE | crates/gcode/src/grbl_file.rs |
| `method EvaluateAddLines` | DONE | crates/gcode/src/grbl_file.rs |
| `method SafeLoadFile` | DONE | crates/gcode/src/grbl_file.rs |
| `method LoadFile` | DONE | crates/gcode/src/grbl_file.rs |
| `method LoadImportedSVG` | DONE | crates/gcode/src/vector.rs |
| `class ColorSegment` | DONE | crates/gcode/src/l2l.rs |
| `property mColor` | DONE | crates/gcode/src/l2l.rs |
| `field mPixLen` | DONE | crates/gcode/src/l2l.rs |
| `ctor ColorSegment` | DONE | crates/gcode/src/l2l.rs |
| `method Fast` | DONE | crates/gcode/src/l2l.rs |
| `method formatnumber` | DONE | crates/gcode/src/grbl_file.rs |
| `method FormatLaserPower` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `class XSegment` | DONE | crates/gcode/src/l2l.rs |
| `ctor XSegment` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `class YSegment` | DONE | crates/gcode/src/l2l.rs |
| `ctor YSegment` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `class DSegment` | DONE | crates/gcode/src/l2l.rs |
| `ctor DSegment` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `class VSeparator` | DONE | crates/gcode/src/l2l.rs |
| `ctor VSeparator` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `class HSeparator` | DONE | crates/gcode/src/l2l.rs |
| `ctor HSeparator` | DONE | crates/gcode/src/l2l.rs |
| `method ToGCodeNumber` | DONE | crates/gcode/src/l2l.rs |
| `method RasterFilling` | DONE | crates/gcode/src/l2l.rs |
| `method VectorFilling` | DONE | crates/gcode/src/l2l.rs |
| `method TimeConsumingFilling` | DONE | crates/gcode/src/l2l.rs |
| `method CheckInUse` | DONE | crates/gcode/src/grbl_file.rs |
| `method LoadImagePotrace` | DONE | crates/gcode/src/vector.rs |
| `method RiseOnFileLoaded` | DONE | crates/gcode/src/grbl_file.rs |
| `method RiseOnFileLoading` | DONE | crates/gcode/src/grbl_file.rs |
| `class L2LConf` | DONE | crates/gcode/src/l2l.rs |
| `field res` | DONE | crates/gcode/src/l2l.rs |
| `field oX` | DONE | crates/gcode/src/l2l.rs |
| `field oY` | DONE | crates/gcode/src/l2l.rs |
| `field markSpeed` | DONE | crates/gcode/src/l2l.rs |
| `field borderSpeed` | DONE | crates/gcode/src/l2l.rs |
| `field minPower` | DONE | crates/gcode/src/l2l.rs |
| `field maxPower` | DONE | crates/gcode/src/l2l.rs |
| `field lOn` | DONE | crates/gcode/src/l2l.rs |
| `field lOff` | DONE | crates/gcode/src/l2l.rs |
| `field dir` | DONE | crates/gcode/src/l2l.rs |
| `field pwm` | DONE | crates/gcode/src/l2l.rs |
| `field fres` | DONE | crates/gcode/src/l2l.rs |
| `field vectorfilling` | DONE | crates/gcode/src/l2l.rs |
| `field firmwareType` | DONE | crates/gcode/src/l2l.rs |
| `field skipcmd` | DONE | crates/gcode/src/l2l.rs |
| `method LoadImageL2L` | DONE | crates/gcode/src/l2l.rs |
| `method GenerateCuttingTest` | DONE | crates/gcode/src/generators.rs |
| `method GenerateGreyscaleTest` | DONE | crates/gcode/src/generators.rs |
| `method GenerateShakeTest` | DONE | crates/gcode/src/generators.rs |
| `method GenerateShakeTest2` | DONE | crates/gcode/src/generators.rs |
| `method ImageLine2Line` | DONE | crates/gcode/src/l2l.rs |
| `method OptimizeLine2Line` | DONE | crates/gcode/src/l2l.rs |
| `method GetSegments` | DONE | crates/gcode/src/l2l.rs |
| `method ExtractSegment` | DONE | crates/gcode/src/l2l.rs |
| `method ParallelOptimizePaths` | DONE | crates/gcode/src/optimize.rs |
| `method GetTaskJob` | DONE | crates/gcode/src/optimize.rs |
| `method OptimizePaths` | DONE | crates/gcode/src/optimize.rs |
| `method SquareDistance` | DONE | crates/gcode/src/optimize.rs |
| `method SquareDistanceZero` | DONE | crates/gcode/src/optimize.rs |
| `method DirectionChange` | N/A | dead code: only called from the commented-out `TravelCost` (grep GrblFile.cs); `OptimizePaths` ignores `changecost` |
| `method GetColor` | DONE | crates/gcode/src/l2l.rs |
| `method formatnumber` | DONE | crates/gcode/src/grbl_file.rs |
| `method IsEven` | DONE | crates/gcode/src/l2l.rs |
| `property EstimatedTime` | DONE | crates/gcode/src/grbl_file.rs |
| `method DrawOnGraphics` | DONE | crates/gcode/src/draw.rs |
| `method DrawJobPreview` | DONE | crates/gcode/src/draw.rs |
| `method LoadImageCenterline` | DONE | crates/gcode/src/vector.rs |
| `method Analyze` | DONE | crates/gcode/src/grbl_file.rs |
| `method ScaleAndPosition` | DONE | crates/gcode/src/draw.rs |
| `method DrawJobRange` | DONE | crates/gcode/src/draw.rs |
| `method GetPen` | DONE | crates/gcode/src/draw.rs |
| `method GetBrush` | DONE | crates/gcode/src/draw.rs |
| `method DrawString` | DONE | crates/gcode/src/draw.rs |
| `method DrawRotatedTextAt` | DONE | crates/gcode/src/draw.rs |
| `method GetEnumerator` | DONE | crates/gcode/src/grbl_file.rs |
| `property Range` | DONE | crates/gcode/src/grbl_file.rs |
| `property InUse` | DONE | crates/gcode/src/grbl_file.rs |
| `class ProgramRange` | DONE | crates/gcode/src/program_range.rs |
| `class XYRange` | DONE | crates/gcode/src/program_range.rs |
| `class Range` | DONE | crates/gcode/src/grbl_file.rs |
| `field Min` | DONE | crates/gcode/src/grbl_file.rs |
| `field Max` | DONE | crates/gcode/src/grbl_file.rs |
| `ctor Range` | DONE | crates/gcode/src/grbl_file.rs |
| `method UpdateRange` | DONE | crates/gcode/src/grbl_file.rs |
| `method ResetRange` | DONE | crates/gcode/src/grbl_file.rs |
| `field X` | DONE | crates/gcode/src/grbl_file.rs |
| `field Y` | DONE | crates/gcode/src/grbl_file.rs |
| `method UpdateRange` | DONE | crates/gcode/src/grbl_file.rs |
| `method UpdateRange` | DONE | crates/gcode/src/grbl_file.rs |
| `method ResetRange` | DONE | crates/gcode/src/grbl_file.rs |
| `class SRange` | DONE | crates/gcode/src/program_range.rs |
| `class Range` | DONE | crates/gcode/src/grbl_file.rs |
| `field Min` | DONE | crates/gcode/src/grbl_file.rs |
| `field Max` | DONE | crates/gcode/src/grbl_file.rs |
| `ctor Range` | DONE | crates/gcode/src/grbl_file.rs |
| `method UpdateRange` | DONE | crates/gcode/src/grbl_file.rs |
| `method ResetRange` | DONE | crates/gcode/src/grbl_file.rs |
| `field S` | DONE | crates/gcode/src/grbl_file.rs |
| `method UpdateRange` | DONE | crates/gcode/src/grbl_file.rs |
| `method ResetRange` | DONE | crates/gcode/src/grbl_file.rs |
| `field DrawingRange` | DONE | crates/gcode/src/program_range.rs |
| `field MovingRange` | DONE | crates/gcode/src/program_range.rs |
| `field SpindleRange` | DONE | crates/gcode/src/program_range.rs |
| `method UpdateXYRange` | DONE | crates/gcode/src/program_range.rs |
| `method UpdateXYRange` | DONE | crates/gcode/src/program_range.rs |
| `method UpdateSRange` | DONE | crates/gcode/src/program_range.rs |
| `method ResetRange` | DONE | crates/gcode/src/grbl_file.rs |

## `Hershey/Hershey.cs` (WP-D)

| member | status | Rust location / reason |
|---|---|---|
| `class Hershey` | DONE | crates/hershey/src/lib.rs |
| `field spbwl` | DONE | `SPBWL` |
| `field hor` | DONE | `data::HOR` (crates/hershey/src/data.rs) |
| `field ver` | DONE | `data::VER` |
| `method CreateString` | DONE | `lg_hershey::create_string` (returns raw lines; caller must Trim+ToUpper like GrblCommand ctor) |
| `method MeasureString` | DONE | `lg_hershey::measure_string` |
| `method ApplyOffset` | DONE | `apply_offset` (private) + `find_coords` replacing the regex |

## `HotKeysManager.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class HotKeysManager` | DONE | crates/services/src/hotkeys.rs |
| `field mCore` | N/A | core/forms are passed as HotKeyContext; effects returned to the UI |
| `field mPreviewForm` | N/A | replaced by HotKeyContext.custom_button_count |
| `field mJogForm` | N/A | UI executes HotKeyEffect::ChangeJogStep/Speed |
| `field mJogKeyRequested` | DONE | crates/services/src/hotkeys.rs |
| `class HotKey` | DONE | crates/services/src/hotkeys.rs |
| `enum Actions` | DONE | crates/services/src/hotkeys.rs |
| `field mCombination` | DONE | crates/services/src/hotkeys.rs |
| `field mAction` | DONE | crates/services/src/hotkeys.rs |
| `ctor HotKey` | DONE | crates/services/src/hotkeys.rs |
| `method ValidShortcut` | DONE | crates/services/src/hotkeys.rs |
| `method IsStandard` | DONE | crates/services/src/hotkeys.rs |
| `method IsNumeric` | DONE | crates/services/src/hotkeys.rs |
| `method IsLetter` | DONE | crates/services/src/hotkeys.rs |
| `method IsFunction` | DONE | crates/services/src/hotkeys.rs |
| `method IsModifier` | N/A | unused private helper |
| `method SetShortcut` | DONE | crates/services/src/hotkeys.rs |
| `method Clone` | DONE | crates/services/src/hotkeys.rs (HotKey is Copy) |
| `ctor HotKeysManager` | DONE | crates/services/src/hotkeys.rs |
| `method SetCustomButtonToolbar` | N/A | empty method in C# |
| `method AddAllFeatures` | DONE | crates/services/src/hotkeys.rs |
| `method AddNew` | DONE | crates/services/src/hotkeys.rs |
| `method Init` | DONE | crates/services/src/hotkeys.rs |
| `method CompareKey` | DONE | crates/services/src/hotkeys.rs |
| `method ManageHotKeys` | DONE | crates/services/src/hotkeys.rs |
| `method Match` | DONE | crates/services/src/hotkeys.rs |
| `method PerformAction` | DONE | crates/services/src/hotkeys.rs |
| `method ChangeJogStep` | DONE | crates/services/src/hotkeys.rs |
| `method ChangeJogSpeed` | DONE | crates/services/src/hotkeys.rs |
| `method RequestJog` | DONE | crates/services/src/hotkeys.rs |
| `method EmulateCustomButtonDown` | DONE | crates/services/src/hotkeys.rs |
| `method EmulateCustomButtonUp` | DONE | crates/services/src/hotkeys.rs |
| `method GetEditList` | DONE | crates/services/src/hotkeys.rs |
| `field cnv` | DONE | crates/services/src/hotkeys.rs keys::to_string |
| `method GetHotKeyString` | DONE | crates/services/src/hotkeys.rs |

## `HotkeyManagerForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class HotkeyManagerForm` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `field mLocalList` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `ctor HotkeyManagerForm` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method OnKeyUp` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method ProcessCmdKey` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method SetNewValue` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method ComputeErrors` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |
| `method BtnSave_Click` | DONE | app/src/forms/dialogs/hotkey_manager_form.rs |

## `Icons/IIconsLoader.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `interface IIconsLoader` | DONE | `ui/icons.rs` (`Icons` active loader, `contains`) |

## `Icons/IconsMgr.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class LoadedImageTag` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property Color` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property Colorize` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property ResourceName` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property Size` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `class IconsMgr` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `field mIconsLoader` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `field mControls` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `field mLightIconColors` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `field mDarkIconColors` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `field mIconColors` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property LegacyIcons` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `property IconsLoader` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method Initialize` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method LoadImage` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method AddControl` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PrepareButton` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PrepareButton` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PrepareButton` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PrepareButton` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PreparePictureBox` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method PrepareMenuItem` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method ControlDisposed` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method OnColorChannge` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |
| `method ReloadLoadImage` | DONE | `ui/icons.rs` (`Icons`, `icons()`, `Tint`, color tables `LIGHT`/`DARK`) |

## `Icons/LegacyIcons.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class LegacyIcons` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |
| `field mIconList` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |
| `ctor LegacyIcons` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |
| `method LoadBase64` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |
| `method LoadImage` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |
| `method Contains` | DONE | `ui/icons.rs` (`Icons::new(true)`, `base64_decode`) |

## `IssueDetectorForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class IssueDetectorForm` | DONE | app/src/forms/dialogs/issue_detector_form.rs |
| `ctor IssueDetectorForm` | DONE | app/src/forms/dialogs/issue_detector_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/issue_detector_form.rs |
| `method LL_LinkClicked` | DONE | app/src/forms/dialogs/issue_detector_form.rs |
| `method BtnOK_Click` | DONE | app/src/forms/dialogs/issue_detector_form.rs |

## `JogForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class JogForm` | DONE | app/src/forms/main/jog.rs |
| `ctor JogForm` | DONE | app/src/forms/main/jog.rs |
| `method SetCore` | DONE | app/src/forms/main/jog.rs |
| `method SettingsForm_SettingsChanged` | DONE | app/src/forms/main/jog.rs |
| `method Core_JogStateChange` | DONE | app/src/forms/main/jog.rs |
| `method OnJogButtonMouseDown` | DONE | app/src/forms/main/jog.rs |
| `method OnJogButtonMouseUp` | DONE | app/src/forms/main/jog.rs |
| `method OnZJogButtonMouseDown` | DONE | app/src/forms/main/jog.rs |
| `method TbSpeed_ValueChanged` | DONE | app/src/forms/main/jog.rs |
| `method TbStep_ValueChanged` | DONE | app/src/forms/main/jog.rs |
| `method ChangeJogStepIndexBy` | DONE | app/src/forms/main/jog.rs |
| `method ChangeJogSpeedIndexBy` | DONE | app/src/forms/main/jog.rs |
| `method UpdateFMax_Tick` | DONE | app/src/forms/main/jog.rs |
| `class StepBar` | DONE | app/src/forms/main/jog.rs |
| `ctor StepBar` | DONE | app/src/forms/main/jog.rs |
| `property CurIndex` | DONE | app/src/forms/main/jog.rs |
| `method ChangeIndexBy` | DONE | app/src/forms/main/jog.rs |
| `class DirectionButton` | DONE | app/src/forms/main/jog.rs |
| `field mDir` | DONE | app/src/forms/main/jog.rs |
| `method OnSizeChanged` | DONE | app/src/forms/main/jog.rs |
| `class DirectionStepButton` | DONE | app/src/forms/main/jog.rs |
| `field mStep` | DONE | app/src/forms/main/jog.rs |
| `method OnSizeChanged` | DONE | app/src/forms/main/jog.rs |

## `LaserLifeEdit.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class LaserLifeEdit` | DONE | app/src/forms/dialogs/laser_life_edit.rs |
| `ctor LaserLifeEdit` | DONE | app/src/forms/dialogs/laser_life_edit.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/laser_life_edit.rs |
| `method BtnSave_Click` | DONE | app/src/forms/dialogs/laser_life_edit.rs |
| `method DiPower_CurrentValueChanged` | DONE | app/src/forms/dialogs/laser_life_edit.rs |
| `method TbLaserName_TextChanged` | DONE | app/src/forms/dialogs/laser_life_edit.rs |

## `LaserSelector.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class LaserSelector` | DONE | app/src/forms/dialogs/laser_selector.rs |
| `ctor LaserSelector` | DONE | app/src/forms/dialogs/laser_selector.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/laser_selector.rs |

## `LaserUsage.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class LaserUsage` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `field mCore` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `field mList` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `ctor LaserUsage` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method LaserUsage_Load` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method RefreshList` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnClose_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method LVLasers_SelectedIndexChanged` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method RefreshButtons` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method GetSelected` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `class LaserLifeCounterLVI` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `field Parent` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `ctor LaserLifeCounterLVI` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnAddNew_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnEdit_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnRemove_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnMark_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnReport_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method LVLasers_DoubleClick` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method BtnUnmark_Click` | DONE | app/src/forms/dialogs/laser_usage.rs |
| `method LaserUsage_Shown` | DONE | app/src/forms/dialogs/laser_usage.rs |

## `LegalDisclaimer.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class LegalDisclaimer` | DONE | app/src/forms/legal_disclaimer.rs |
| `field accepted` | DONE | app/src/forms/legal_disclaimer.rs |
| `ctor LegalDisclaimer` | DONE | app/src/forms/legal_disclaimer.rs |
| `method LlTranslate_LinkClicked` | DONE | app/src/forms/legal_disclaimer.rs |
| `method BtnAccept_Click` | DONE | app/src/forms/legal_disclaimer.rs |
| `method CbIHaveRead_CheckedChanged` | DONE | app/src/forms/legal_disclaimer.rs |

## `LicenseForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class LicenseForm` | DONE | app/src/forms/license.rs |
| `ctor LicenseForm` | DONE | app/src/forms/license.rs |
| `method BtnContinue_Click` | DONE | app/src/forms/license.rs |
| `method BtnDonate_Click` | DONE | app/src/forms/license.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/license.rs |
| `method RTBLinkClick` | DONE | app/src/forms/license.rs |

## `Logger/AsyncLogFile.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class AsyncLogFile` | DONE | crates/base/src/logger/async_file.rs |
| `field Q` | DONE | crates/base/src/logger/async_file.rs |
| `field EV` | DONE | crates/base/src/logger/async_file.rs |
| `field TH` | DONE | crates/base/src/logger/async_file.rs |
| `field PID` | N/A | assigned but never used in C# |
| `field Exit` | DONE | crates/base/src/logger/async_file.rs |
| `field mFilename` | DONE | crates/base/src/logger/async_file.rs |
| `ctor AsyncLogFile` | DONE | crates/base/src/logger/async_file.rs |
| `method Start` | DONE | crates/base/src/logger/async_file.rs |
| `method RotateLog` | DONE | crates/base/src/logger/async_file.rs |
| `method Stop` | DONE | crates/base/src/logger/async_file.rs |
| `method Log` | DONE | crates/base/src/logger/async_file.rs |
| `method Enqueue` | DONE | crates/base/src/logger/async_file.rs |
| `method DoTheWork` | DONE | crates/base/src/logger/async_file.rs |
| `method ShowLog` | DONE | crates/base/src/logger/async_file.rs |

## `Logger/ComLogger.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class ComLogger` | DONE | crates/base/src/logger/com.rs |
| `field lockstr` | DONE | crates/base/src/logger/com.rs |
| `field file` | DONE | crates/base/src/logger/com.rs |
| `field logcnt` | DONE | crates/base/src/logger/com.rs |
| `field logid` | DONE | crates/base/src/logger/com.rs |
| `method StartLog` | DONE | crates/base/src/logger/com.rs |
| `method StopLog` | DONE | crates/base/src/logger/com.rs |
| `property Enabled` | DONE | crates/base/src/logger/com.rs |
| `method Log` | DONE | crates/base/src/logger/com.rs |
| `method Log` | DONE | crates/base/src/logger/com.rs |
| `method Log` | DONE | crates/base/src/logger/com.rs |

## `Logger/Logger.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class Logger` | DONE | crates/base/src/logger/mod.rs |
| `field file` | DONE | crates/base/src/logger/mod.rs |
| `method LogException` | DONE | crates/base/src/logger/mod.rs |
| `method LogMessage` | DONE | crates/base/src/logger/mod.rs |
| `method Start` | DONE | crates/base/src/logger/mod.rs |
| `method Stop` | DONE | crates/base/src/logger/mod.rs |
| `method LogMultiLine` | DONE | crates/base/src/logger/mod.rs |
| `method Log` | DONE | crates/base/src/logger/mod.rs |
| `method ShowLog` | DONE | crates/base/src/logger/mod.rs |

## `MainForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class MainForm` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field Core` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field ToolBarMessage` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field IsBufferStuck` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field MultiRunShown` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field PromptForWiFiShown` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field args` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field mLineWidthMenu` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `ctor MainForm` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `ctor MainForm` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnAutoUpdateDropDown_Closing` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method RefreshColorSchema` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method AssignIcons` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MainForm_Load` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method IsVisibleOnAnyScreen` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method ManageCommandLineArgs` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method ManageMessage` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method UpdateTimer_Tick` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method PromptForWiFi` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method ShowWiFiConfig` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method TimerUpdate` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method IsConfiguredForWiFi` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method RefreshFormTitle` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method RefreshMenuHotKeys` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnFileOpen_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnFileSend_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnGrblReset_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method GetOvMenu` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `class ToolStripTraceBarItem` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `ctor ToolStripTraceBarItem` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnGoHome_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnUnlock_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnConnect_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnDisconnect_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnAdvancedSave_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnSaveProject_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNEnglish_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNItalian_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNSpanish_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method SetLanguage` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method helpOnLineToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method aboutToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method splitContainer1_SplitterMoved` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method settingsToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method openSessionLogToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method toolStripMenuItem4_DropDownOpening` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNFrench_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNGerman_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNDanish_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNBrazilian_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method russianToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method chineseToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method slovakToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MNGrblEmulator_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method styleToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method grblConfigurationToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method donateToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnKeyUp` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method ProcessCmdKey` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnReOpenFile_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method fileToolStripMenuItem_DropDownOpening` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnHotkeys_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method AwakeTimer_Tick` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnStartFromPosition_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnFileAppend_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method hungarianToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method czechToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method installCH340DriverToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method flashGrblFirmwareToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method toolsToolStripMenuItem_DropDownOpening` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method activateExtendedLogToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field dropDispatcherTimer` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `field droppedFile` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MainForm_DragEnter` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MainForm_DragDrop` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method licenseToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnNotifyNewVersion_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnNotifyNewVersion_CheckedChanged` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnNotifyMinorVersion_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnNotifyPreRelease_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnCheckNow_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnMaterialDB_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method polishToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method orturSupportGroupToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method orturWebsiteToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method traditionalChineseToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method youtubeChannelToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method firmwareToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method manualsDownloadToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MultipleInstanceTimer_Tick` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnRunMulti_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method orturSupportAndFeedbackToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method TTLinkToNews_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnWiFiDiscovery_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method facebookCommunityToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method greekToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method turkishToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method BtnUnlockFromStuck_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method romanianToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method dutchToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method ukrainianToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method japaneseToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method TTTStatus_DoubleClick` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnConfigureOrturWiFi_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method TTlaserLife_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method laserUsageStatsToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnGrayscaleTest_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnCuttingTest_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnAccuracyTest_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method shakeTestToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method autoSizeToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method zoomInToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method zoomOutToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method showLaserOffMovementsToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method showExecutedCommandsToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method pxToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method CheckLineWidthItem` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method SplitContainer1_Paint` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method showDiagnosticDataToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method showBoundingBoxToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method crossCursorToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method drawingAreaToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method machineAreaToolStripMenuItem_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnConfigureLongerWiFi_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerUserGroup_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerSupportAndFeedback_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerWebsite_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerYouTube_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerManuals_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method MnLongerFirmware_Click` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `class MMnRenderer` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `ctor MMnRenderer` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderMenuItemBackground` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderImageMargin` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderItemBackground` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderItemText` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderToolStripBackground` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderToolStripContentPanelBackground` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderToolStripPanelBackground` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `method OnRenderToolStripBorder` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |
| `class CustomMenuColor` | DONE | app/src/main_form.rs, app/src/forms/main/{menus,status_bar,dialogs}.rs |

## `MyDatagridView.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `method OnHandleCreated` | N/A | WinForms DataGridView crash workaround; egui table helper `app/src/ui/widgets/data_grid.rs` (`data_grid`) instead |

## `Obj3D/ColorHelper.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class ColorHelper` | DONE | `app/src/ui/preview/scene.rs` (`color_helper_blend`) |
| `method Blend` | DONE | `app/src/ui/preview/scene.rs` (`color_helper_blend`) |

## `Obj3D/GlColorHelper.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class GlColorHelper` | DONE | `app/src/ui/preview/scene.rs` (`gl_color_from`) |
| `method FromColor` | DONE | `app/src/ui/preview/scene.rs` (`gl_color_from`) |

## `Obj3D/Objects3D.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class Object3DVertex` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property X` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property Y` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property Z` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property Intensity` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property Command` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `class Object3DDisplayList` | N/A | replaced by instanced buffers per `Chunk` |
| `property IsValid` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property DisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property Vertices` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Begin` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method End` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Invalidate` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Invalidated` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `class Object3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mDisplayLists` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property LineWidth` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mGL` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mCurrentDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field MAX_VECTOR_IN_DISPLAY_LIST` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mInvalidate` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property VertexCounter` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `ctor Object3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method ClearDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Render` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method CallDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method CreateDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Draw` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method NewDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method AddVertex` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method CheckListSize` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Dispose` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Invalidate` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `class Grid3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property ViewportSize` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property ControlWidth` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mDisplayTick` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mDisplayMinor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mCamera` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mTicksColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property TicksColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mMinorsColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mOriginsColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property GridSize` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property mShowMinor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `ctor Grid3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method DrawVerticalLine` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method DrawHorizontalLine` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Draw` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method CallDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `class Grbl3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field Core` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mJustLaserOffMovements` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mBoundingBoxDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field BackgroundColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field Color` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field BoundingBoxColor` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field mLoadingPercentage` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `property ShowBoundingBox` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field BOUNDING_RECT_LINE_WIDTH` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field BOUNDING_RECT_STIPPLE` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `delegate void` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field OnLoadingPercentageChange` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `ctor Grbl3D` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Map` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Blend` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Draw` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field zPos` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `field borderLimit` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method Invalidate` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method InvalidateAll` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |
| `method CallDisplayList` | DONE | `app/src/ui/preview/scene.rs` (`Chunk`, `Layer`, `build_scene`, `recolor_chunk`), `gl.rs` (instanced-quad renderer), `panel_gl.rs` (`grid_batches`, `bbox_batch`, `pointer_batch`) |

## `PSHelper/MaterialDB.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class MaterialDB` | DONE | crates/services/src/material_db.rs |
| `class MaterialsDataTable` | DONE | crates/services/src/material_db.rs |
| `method EndInit` | DONE | crates/services/src/material_db.rs |
| `method MaterialsDataTable_TableNewRow` | DONE | crates/services/src/material_db.rs |
| `method OnTableNewRow` | DONE | crates/services/src/material_db.rs |
| `method GetLastNotDeleted` | DONE | crates/services/src/material_db.rs |
| `method OnColumnChanging` | DONE | crates/services/src/material_db.rs |
| `property EnabledRows` | DONE | crates/services/src/material_db.rs |
| `method Models` | DONE | crates/services/src/material_db.rs |
| `method Materials` | DONE | crates/services/src/material_db.rs |
| `method Thickness` | DONE | crates/services/src/material_db.rs |
| `method Actions` | DONE | crates/services/src/material_db.rs |
| `method GetResult` | DONE | crates/services/src/material_db.rs |
| `field FromServer` | DONE | crates/services/src/material_db.rs |
| `method Load` | DONE | crates/services/src/material_db.rs |
| `method SaveChanges` | DONE | crates/services/src/material_db.rs |
| `method GetNewCount` | DONE | crates/services/src/material_db.rs |
| `method ImportServer` | DONE | crates/services/src/material_db.rs |
| `property UserFile` | DONE | crates/services/src/material_db.rs |
| `property ServerFile` | DONE | crates/services/src/material_db.rs |

## `PSHelper/PSEditorForm.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class PSEditorForm` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `ctor PSEditorForm` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method CreateAndShowDialog` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method PSEditorForm_FormClosing` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method BtnImport_Click` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method DG_DataError` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method DG_CellParsing` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method StripNonNumber` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method Paste` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method GetInt` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |
| `method DG_KeyDown` | DONE | app/src/forms/material/ps_editor.rs (`MaterialEditor`) |

## `PSHelper/PSHelperForm.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class PSHelperForm` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `ctor PSHelperForm` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method SetDefault` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method CreateAndShowDialog` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method InitModels` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method CbModel_SelectedIndexChanged` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `property SelectedModel` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `property SelectedMaterial` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `property SelectedAction` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `property SelectedThickness` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method CbMaterial_SelectedIndexChanged` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method CbAction_SelectedIndexChanged` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method CbThickness_SelectedIndexChanged` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method BtnApply_Click` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |
| `method BtnCancel_Click` | DONE | app/src/forms/material/ps_helper.rs (`PsHelperForm`) |

## `PreviewForm.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class PreviewForm` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field mCore` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `property GrblPanel` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `ctor PreviewForm` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method CreatePanel` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method CustomButtonArea_OrderChanged` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method SetCore` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method TimerUpdate` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method BtnUnlockClick` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method addCustomButtonToolStripMenuItem_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method RefreshCustomButtons` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `class CustomButtonIB` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field Core` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field cb` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field tt` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field cms` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field mDrawDisabled` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `ctor CustomButtonIB` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method Dispose` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method RefreshEnabled` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnPaint` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method DrawDisabled` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnClick` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field mEmulateMouseInside` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method IsMouseInside` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field on` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method PerformClick` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnMouseDown` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method PerformMouseDown` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnMouseUp` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method PerformMouseUp` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method RemoveButton_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method EditButton_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field _isDragging` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field _DDradius` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field _mX` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field _mY` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnMouseMove` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `class MyFlowPanel` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `delegate void` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `field OrderChanged` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `ctor MyFlowPanel` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnControlAdded` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnControlRemoved` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnPaintBackground` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnDragEnter` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `property ButtonPositionUnlocked` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnDragOver` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnDragDrop` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnDragLeave` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method OnColorChange` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method BtnZeroing_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method exportCustomButtonsToolStripMenuItem_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method importCustomButtonsToolStripMenuItem_Click` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method ImportButton` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method MNAddCB_Opening` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |
| `method MnUnlockButtonClick` | DONE | app/src/forms/preview_form/mod.rs, custom_button_ib.rs |

## `Program.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class Program` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `field DesignTime` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `property CurrentVersion` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method Main` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method FixTLS` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method bypassAllCertificateStuff` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `class GraphicInitializer` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method LoadNvApi64` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method LoadNvApi32` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |
| `method RequestDedicatedGraphics` | DONE | app/src/main.rs (args, logger, panic hook, geometry) + app/src/app_state.rs; NvAPI/TLS/TaskScheduler parts N/A on Linux |

## `RasterConverter/ConvertSizeAndOptionForm.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class ConvertSizeAndOptionForm` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `field ratiolock` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `field LaserOptions` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `class ComboboxItem` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `property Text` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `property Value` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `ctor ComboboxItem` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method ToString` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `ctor ConvertSizeAndOptionForm` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method AssignMinMaxLimit` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method ShowDialog` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method InitImageSize` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method IISizeW_CurrentValueChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method IISizeH_CurrentValueChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method RefreshPerc` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnOnOffInfo_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnModulationInfo_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method CBLaserON_SelectedIndexChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method IISizeW_OnTheFlyValueChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method IISizeH_OnTheFlyValueChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method CbAutosize_CheckedChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method IIDpi_CurrentValueChanged` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method ComputeDpiSize` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnDPI_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnPSHelper_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnCreate_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method ConfirmOutOfBoundary` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnReset_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnCenter_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |
| `method BtnUnlockProportion_Click` | DONE | app/src/forms/raster/convert_size.rs (`ConvertSizeForm`) |

## `RasterConverter/Dithering/ArgbColor.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `struct ArgbColor` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `field B` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `field G` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `field R` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `field A` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `ctor ArgbColor` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |
| `method FromArgb` | DONE | crates/raster/src/dithering/argb_color.rs `ArgbColor` |

## `RasterConverter/Dithering/AtkinsonDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class AtkinsonDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::atkinson` |
| `ctor AtkinsonDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::atkinson` |

## `RasterConverter/Dithering/BurkesDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class BurksDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::burks` |
| `ctor BurksDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::burks` |

## `RasterConverter/Dithering/ErrorDiffusionDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class ErrorDiffusionDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _divisor` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _matrix` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _matrixHeight` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _matrixWidth` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _startingOffset` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `field _useShifting` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `ctor ErrorDiffusionDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |
| `method ToByte` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering` (+ `to_byte`) |

## `RasterConverter/Dithering/FloydSteinbergDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class FloydSteinbergDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::floyd_steinberg` |
| `ctor FloydSteinbergDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::floyd_steinberg` |

## `RasterConverter/Dithering/IErrorDiffusion.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `interface IErrorDiffusion` | DONE | crates/raster/src/dithering/error_diffusion.rs trait `ErrorDiffusion` |

## `RasterConverter/Dithering/ImageUtilities.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class ImageUtilities` | DONE | crates/raster/src/dithering/image_utilities.rs |
| `method Copy` | DONE | crates/raster/src/dithering/image_utilities.rs |
| `method ToBitmap` | DONE | crates/raster/src/dithering/image_utilities.rs |
| `method GetPixelsFrom32BitArgbImage` | DONE | crates/raster/src/dithering/image_utilities.rs |

## `RasterConverter/Dithering/JarvisJudiceNinkeDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class JarvisJudiceNinkeDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::jarvis_judice_ninke` |
| `ctor JarvisJudiceNinkeDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::jarvis_judice_ninke` |

## `RasterConverter/Dithering/RandomDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class RandomDithering` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` / `DotNetRandom` / `tick_count` |
| `field _black` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` fields |
| `field _blackDefault` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` fields |
| `field _random` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` fields |
| `field _white` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` fields |
| `field _whiteDefault` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` fields |
| `ctor RandomDithering` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` / `DotNetRandom` / `tick_count` |
| `ctor RandomDithering` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` / `DotNetRandom` / `tick_count` |
| `ctor RandomDithering` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` / `DotNetRandom` / `tick_count` |
| `ctor RandomDithering` | DONE | crates/raster/src/dithering/random.rs `RandomDithering` / `DotNetRandom` / `tick_count` |

## `RasterConverter/Dithering/Sierra2Dithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class Sierra2Dithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra2` |
| `ctor Sierra2Dithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra2` |

## `RasterConverter/Dithering/Sierra3Dithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class Sierra3Dithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra3` |
| `ctor Sierra3Dithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra3` |

## `RasterConverter/Dithering/SierraLiteDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class SierraLiteDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra_lite` |
| `ctor SierraLiteDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::sierra_lite` |

## `RasterConverter/Dithering/StuckiDithering.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class StuckiDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::stucki` |
| `ctor StuckiDithering` | DONE | crates/raster/src/dithering/error_diffusion.rs `ErrorDiffusionDithering::stucki` |

## `RasterConverter/ImageProcessor.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class ImageProcessor` | DONE | crates/services/src/image_processor/ |
| `delegate void` | DONE | crates/services/src/image_processor/ |
| `field PreviewBegin` | DONE | crates/services/src/image_processor/ |
| `delegate void` | DONE | crates/services/src/image_processor/ |
| `field PreviewReady` | DONE | crates/services/src/image_processor/ |
| `delegate void` | DONE | crates/services/src/image_processor/ |
| `field GenerationComplete` | DONE | crates/services/src/image_processor/ |
| `field mTrueOriginal` | DONE | crates/services/src/image_processor/ |
| `field mOriginal` | DONE | crates/services/src/image_processor/ |
| `field mResized` | DONE | crates/services/src/image_processor/ |
| `field mFileDPI` | DONE | crates/services/src/image_processor/ |
| `field mFileResolution` | DONE | crates/services/src/image_processor/ |
| `field mGrayScale` | DONE | crates/services/src/image_processor/ |
| `field mSuspended` | DONE | crates/services/src/image_processor/ |
| `field mBoxSize` | DONE | crates/services/src/image_processor/ |
| `field mInterpolation` | DONE | crates/services/src/image_processor/ |
| `field mTool` | DONE | crates/services/src/image_processor/ |
| `field mFormula` | DONE | crates/services/src/image_processor/ |
| `field mRed` | DONE | crates/services/src/image_processor/ |
| `field mGreen` | DONE | crates/services/src/image_processor/ |
| `field mBlue` | DONE | crates/services/src/image_processor/ |
| `field mContrast` | DONE | crates/services/src/image_processor/ |
| `field mWhitePoint` | DONE | crates/services/src/image_processor/ |
| `field mBrightness` | DONE | crates/services/src/image_processor/ |
| `field mThreshold` | DONE | crates/services/src/image_processor/ |
| `field mUseThreshold` | DONE | crates/services/src/image_processor/ |
| `field mQuality` | DONE | crates/services/src/image_processor/ |
| `field mLinePreview` | DONE | crates/services/src/image_processor/ |
| `field mSpotRemoval` | DONE | crates/services/src/image_processor/ |
| `field mUseSpotRemoval` | DONE | crates/services/src/image_processor/ |
| `field mOptimize` | DONE | crates/services/src/image_processor/ |
| `field mUseOptimize` | DONE | crates/services/src/image_processor/ |
| `field mUseAdaptiveQuality` | DONE | crates/services/src/image_processor/ |
| `field mSmoothing` | DONE | crates/services/src/image_processor/ |
| `field mUseSmootihing` | DONE | crates/services/src/image_processor/ |
| `field mDownSampling` | DONE | crates/services/src/image_processor/ |
| `field mUseDownSampling` | DONE | crates/services/src/image_processor/ |
| `field mOptimizeFast` | DONE | crates/services/src/image_processor/ |
| `field mDirection` | DONE | crates/services/src/image_processor/ |
| `field mFillingDirection` | DONE | crates/services/src/image_processor/ |
| `field mDithering` | DONE | crates/services/src/image_processor/ |
| `field mFillingQuality` | DONE | crates/services/src/image_processor/ |
| `field mUseLineThreshold` | DONE | crates/services/src/image_processor/ |
| `field mLineThreshold` | DONE | crates/services/src/image_processor/ |
| `field mUseCornerThreshold` | DONE | crates/services/src/image_processor/ |
| `field mCornerThreshold` | DONE | crates/services/src/image_processor/ |
| `field mDemo` | DONE | crates/services/src/image_processor/ |
| `field TargetSize` | DONE | crates/services/src/image_processor/ |
| `field TargetOffset` | DONE | crates/services/src/image_processor/ |
| `field LaserOn` | DONE | crates/services/src/image_processor/ |
| `field LaserOff` | DONE | crates/services/src/image_processor/ |
| `field BorderSpeed` | DONE | crates/services/src/image_processor/ |
| `field MarkSpeed` | DONE | crates/services/src/image_processor/ |
| `field MinPower` | DONE | crates/services/src/image_processor/ |
| `field MaxPower` | DONE | crates/services/src/image_processor/ |
| `field mFileName` | DONE | crates/services/src/image_processor/ |
| `field mAppend` | DONE | crates/services/src/image_processor/ |
| `field Current` | DONE | crates/services/src/image_processor/ |
| `field MustExit` | DONE | crates/services/src/image_processor/ |
| `enum Tool` | DONE | crates/services/src/image_processor/ |
| `enum Direction` | DONE | crates/services/src/image_processor/ |
| `ctor ImageProcessor` | DONE | crates/services/src/image_processor/ |
| `method FormResize` | DONE | crates/services/src/image_processor/ |
| `method Clone` | DONE | crates/services/src/image_processor/ |
| `method Dispose` | DONE | crates/services/src/image_processor/ |
| `method Suspend` | DONE | crates/services/src/image_processor/ |
| `method Resume` | DONE | crates/services/src/image_processor/ |
| `method AutoTrim` | DONE | crates/services/src/image_processor/ |
| `method FindLimit` | DONE | crates/services/src/image_processor/ |
| `method GuessTrimColor` | DONE | crates/services/src/image_processor/ |
| `method GetLineColor` | DONE | crates/services/src/image_processor/ |
| `method ColorAVG` | DONE | crates/services/src/image_processor/ |
| `method IsSimilarColor` | DONE | crates/services/src/image_processor/ |
| `method CropImage` | DONE | crates/services/src/image_processor/ |
| `method Fill` | DONE | crates/services/src/image_processor/ |
| `method Outliner` | DONE | crates/services/src/image_processor/ |
| `method Invert` | DONE | crates/services/src/image_processor/ |
| `method RotateCW` | DONE | crates/services/src/image_processor/ |
| `method RotateCCW` | DONE | crates/services/src/image_processor/ |
| `method FlipH` | DONE | crates/services/src/image_processor/ |
| `method Revert` | DONE | crates/services/src/image_processor/ |
| `method FlipV` | DONE | crates/services/src/image_processor/ |
| `method ResizeRecalc` | DONE | crates/services/src/image_processor/ |
| `method Refresh` | DONE | crates/services/src/image_processor/ |
| `method RunThread` | DONE | crates/services/src/image_processor/ |
| `method AbortThread` | DONE | crates/services/src/image_processor/ |
| `method PreviewCenterline` | DONE | crates/services/src/image_processor/ |
| `method PreviewDithering` | DONE | crates/services/src/image_processor/ |
| `method GenerateGCode` | DONE | crates/services/src/image_processor/ |
| `method GenerateGCode2` | DONE | crates/services/src/image_processor/ |
| `method GetVectorQuality` | DONE | crates/services/src/image_processor/ |
| `method CreateTarget` | DONE | crates/services/src/image_processor/ |
| `method ProduceBitmap` | DONE | crates/services/src/image_processor/ |
| `method ProduceWhitepointDemo` | DONE | crates/services/src/image_processor/ |
| `method ProduceBitmap2` | DONE | crates/services/src/image_processor/ |
| `method PreviewLineByLine` | DONE | crates/services/src/image_processor/ |
| `method PreviewNoProcessing` | DONE | crates/services/src/image_processor/ |
| `method PreviewVector` | DONE | crates/services/src/image_processor/ |
| `method WidthToHeight` | DONE | crates/services/src/image_processor/ |
| `method HeightToWidht` | DONE | crates/services/src/image_processor/ |
| `method CalculateResizeToFit` | DONE | crates/services/src/image_processor/ |
| `property Original` | DONE | crates/services/src/image_processor/ |
| `property TrueOriginal` | DONE | crates/services/src/image_processor/ |
| `property FileDPI` | DONE | crates/services/src/image_processor/ |

## `RasterConverter/ImageTransform.cs` (WP-F)

| member | status | Rust location / reason |
|---|---|---|
| `class ImageTransform` | DONE | crates/raster/src/image_transform.rs (module) |
| `method ResizeImage` | DONE | crates/raster/src/resize.rs `resize_image` (approximation of GDI+) |
| `method Threshold` | DONE | crates/raster/src/image_transform.rs `threshold` |
| `method draw_adjusted_image` | DONE | crates/raster/src/image_transform.rs `draw_adjusted_image` + crates/raster/src/color_matrix.rs `apply_color_matrix` |
| `method InvertingImage` | DONE | crates/raster/src/image_transform.rs `inverting_image` |
| `enum Formula` | DONE | crates/raster/src/image_transform.rs `Formula` |
| `enum DitheringMode` | DONE | crates/raster/src/image_transform.rs `DitheringMode` |
| `method GetDitheringMode` | DONE | crates/raster/src/image_transform.rs `get_dithering_mode` |
| `method DitherImage` | DONE | crates/raster/src/image_transform.rs `dither_image`, `dither_image_with_seed` |
| `method TransformPixel` | DONE | crates/raster/src/image_transform.rs `transform_pixel` |
| `method GrayScale` | DONE | crates/raster/src/image_transform.rs `gray_scale`, `gray_scale_mode`, `gray_scale_matrix` |
| `method Whitenize` | DONE | crates/raster/src/image_transform.rs `whitenize` |
| `method ColorSubstitution` | DONE | crates/raster/src/image_transform.rs `color_substitution` |
| `method Format32bppArgbCopy` | DONE | crates/raster/src/image_transform.rs `format32bpp_argb_copy` (unused upstream) |
| `class DirectBitmap` | DONE | crates/raster/src/image_transform.rs `DirectBitmap` (premultiplied i32 bits) |
| `property Bitmap` | N/A | Rust `DirectBitmap::to_image()` replaces the GDI Bitmap wrapper |
| `property Bits` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::bits` |
| `property Disposed` | N/A | no unmanaged handle; Rust Drop |
| `property Height` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::height` |
| `property Width` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::width` |
| `property BitsHandle` | N/A | GCHandle pinning not needed in Rust |
| `ctor DirectBitmap` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::new` |
| `method SetPixel` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::set_pixel` |
| `method GetPixel` | DONE | crates/raster/src/image_transform.rs `DirectBitmap::get_pixel` |
| `method Dispose` | N/A | Rust Drop |
| `method Fill` | DONE | crates/raster/src/image_transform.rs `fill` |
| `method Fill4` | DONE | crates/raster/src/image_transform.rs `fill4` |
| `method FillColorCompare` | DONE | crates/raster/src/image_transform.rs `fill_color_compare` |
| `method FillChkIn` | DONE | crates/raster/src/image_transform.rs `fill_chk_in` |
| `method Outliner` | DONE | crates/raster/src/image_transform.rs `outliner` |
| `method Outline` | DONE | crates/raster/src/image_transform.rs `outline` |
| `class ColorSubstitutionFilter` | N/A | folded into `color_substitution` parameters |
| `field thresholdValue` | N/A | folded into `color_substitution` parameter `th` |
| `field sourceColor` | N/A | folded into `color_substitution` parameter `source` |
| `field newColor` | N/A | folded into `color_substitution` parameter `new` |

## `RasterConverter/RasterToLaserForm.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class RasterToLaserForm` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `ctor RasterToLaserForm` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method GetMaxQuality` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method GetImageSize` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CreatePaper` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CreateAndShowDialog` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method StoreSettings` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method LoadSettings` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method GetActualSettings` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbMode_SelectedIndexChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBRed_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBGreen_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBBlue_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TbBright_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TbContrast_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbThreshold_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RefreshVE` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TbThreshold_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbLineToLineTracing_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbNoProcessing_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbCenterline_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbVectorize_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDQuality_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbLinePreview_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDSpotRemoval_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbSpotRemoval_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDSmoothing_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbSmoothing_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDOptimize_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbOptimize_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RasterToLaserForm_Load` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbFillingDirection_SelectedIndexChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDFillingQuality_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnOutliner_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbDithering_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbDownSample_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method UDDownSample_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbOptimizeFast_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method PbConverted_Resize` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbDither_SelectedIndexChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnQualityInfo_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnFillingQualityInfo_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBWhiteClip_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBWhiteClip_MouseDown` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBWhiteClip_MouseUp` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnReverse_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbUseLineThreshold_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbCornerThreshold_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBLineThreshold_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBCornerThreshold_ValueChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBCornerThreshold_DoubleClick` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method TBLineThreshold_DoubleClick` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method CbAdaptiveQuality_CheckedChanged` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnAdaptiveQualityInfo_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method BtnAutoTrim_Click` | DONE | app/src/forms/raster/raster_to_laser.rs (`RasterImport`) |
| `method RbCenterline_Click` | N/A | 64-bit warning: the Rust build is always 64-bit |
| `method RbLineToLineTracing_Click` | N/A | PWM warning: radio is hidden when PWM is unsupported, handler unreachable |

## `RasterConverter/ResolutionHelperForm.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class ResolutionHelperForm` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `ctor ResolutionHelperForm` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method GetMaxQuality` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method CreateAndShowDialog` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method BtnCreate_Click` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method Compute` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method MaxRes` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method MinRes` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |
| `method linkLabel1_LinkClicked` | DONE | app/src/forms/raster/resolution_helper.rs (`ResolutionHelperForm`) |

## `Resources/ResourceHelper.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class ResourceHelper` | N/A | resx lookup replaced by ui::i18n (tr) and include_bytes! |
| `method IsEmbeddedResource` | N/A | resx lookup replaced by ui::i18n (tr) and include_bytes! |
| `method GetFileAsStream` | N/A | resx lookup replaced by ui::i18n (tr) and include_bytes! |
| `method GetFileAsText` | N/A | resx lookup replaced by ui::i18n (tr) and include_bytes! |

## `ResumeJobForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class ResumeJobForm` | DONE | app/src/forms/dialogs/resume_job_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/resume_job_form.rs |
| `ctor ResumeJobForm` | DONE | app/src/forms/dialogs/resume_job_form.rs |
| `method RbCheckedChanged` | DONE | app/src/forms/dialogs/resume_job_form.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/dialogs/resume_job_form.rs |
| `method BtnOK_Click` | DONE | app/src/forms/dialogs/resume_job_form.rs |

## `RunFromPositionForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class RunFromPositionForm` | DONE | app/src/forms/dialogs/run_from_position_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/run_from_position_form.rs |
| `ctor RunFromPositionForm` | DONE | app/src/forms/dialogs/run_from_position_form.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/dialogs/run_from_position_form.rs |
| `method BtnOK_Click` | DONE | app/src/forms/dialogs/run_from_position_form.rs |

## `SafetyCountdown.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class SafetyCountdown` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `field player` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `ctor SafetyCountdown` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `ctor SafetyCountdown` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method EngravingStarting_Load` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method TimCountDown_Tick` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method BtnStart_Click` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method ExitOK` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method ExitKO` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method DoBeep` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method EngravingStarting_FormClosing` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method EngravingStarting_Shown` | DONE | app/src/forms/dialogs/safety_countdown.rs |
| `method CanGo` | DONE | app/src/forms/dialogs/safety_countdown.rs |

## `SaveOptionForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class SaveOptionForm` | DONE | app/src/forms/save_options.rs |
| `ctor SaveOptionForm` | DONE | app/src/forms/save_options.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/save_options.rs |

## `Settings.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class Settings` | DONE | crates/base/src/settings.rs |
| `field Timer` | DONE | crates/base/src/settings.rs |
| `field dic` | DONE | crates/base/src/settings.rs |
| `field LastCause` | DONE | crates/base/src/settings.rs |
| `field LockString` | DONE | crates/base/src/settings.rs |
| `property IsNewFile` | DONE | crates/base/src/settings.rs |
| `property PrevVersion` | DONE | crates/base/src/settings.rs |
| `enum GraphicMode` | DONE | crates/base/src/settings.rs |
| `property ForcedGraphicMode` | DONE | crates/base/src/settings.rs |
| `property RequestedGraphicMode` | DONE | crates/base/src/settings.rs |
| `property CurrentGraphicMode` | DONE | crates/base/src/settings.rs |
| `ctor Settings` | DONE | crates/base/src/settings.rs |
| `method GetAndDeleteObject` | DONE | crates/base/src/settings.rs |
| `method SetObject` | DONE | crates/base/src/settings.rs |
| `method TriggerSave` | DONE | crates/base/src/settings.rs |
| `method Exiting` | DONE | crates/base/src/settings.rs |
| `method OnTimerExpire` | DONE | crates/base/src/settings.rs |
| `method InternalSaveFile` | DONE | crates/base/src/settings.rs |
| `method DeleteObject` | DONE | crates/base/src/settings.rs |
| `method ExistObject` | DONE | crates/base/src/settings.rs |

## `SettingsForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class SettingsForm` | DONE | app/src/forms/dialogs/settings_form.rs |
| `field Core` | DONE | app/src/forms/dialogs/settings_form.rs |
| `field SettingsChanged` | DONE | app/src/forms/dialogs/settings_form.rs |
| `field PrevGraphicMode` | DONE | app/src/forms/dialogs/settings_form.rs |
| `ctor SettingsForm` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitAutoCoolingTab` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitCoreCB` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitThreadingCB` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitProtocolCB` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitStreamingCB` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method InitGraphicModeCB` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnSave_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method MaxTs` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnCancel_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnModulationInfo_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnLaserMode_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnProtocol_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnStreamingMode_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnThreadingModel_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnFType_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method changeSucBtn_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method changeWarBtn_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method changeFatBtn_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method changeConBtn_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method changeDconBtn_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method TbNotification_TextChanged` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method EnableTest` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnTestNotification_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method BtnTelegNoteInfo_Click` | DONE | app/src/forms/dialogs/settings_form.rs |
| `method CbTelegramNotification_CheckedChanged` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `method BtnRenderingMode_Click` | DONE | app/src/forms/dialogs/settings_form.rs |

## `SincroStart.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class SincroStart` | DONE | crates/core/src/sincro_start.rs |
| `field EX` | DONE | crates/core/src/sincro_start.rs |
| `field EV` | DONE | crates/core/src/sincro_start.rs |
| `field TH` | DONE | crates/core/src/sincro_start.rs |
| `field C` | DONE | crates/core/src/sincro_start.rs |
| `method StartListen` | DONE | crates/core/src/sincro_start.rs |
| `method Running` | DONE | crates/core/src/sincro_start.rs |
| `method StopListen` | DONE | crates/core/src/sincro_start.rs |
| `method DoTheJob` | DONE | crates/core/src/sincro_start.rs |
| `method Signal` | DONE | crates/core/src/sincro_start.rs |
| `method SignalEvent` | DONE | crates/core/src/sincro_start.rs |

## `Sound/SoundEvent.cs` (WP-C1a)

| member | status | Rust location / reason |
|---|---|---|
| `class SoundEvent` | DONE | crates/base/src/sound.rs |
| `enum EventId` | DONE | crates/base/src/sound.rs |
| `field mBusy` | DONE | crates/base/src/sound.rs |
| `field mLock` | DONE | crates/base/src/sound.rs |
| `method PlaySound` | DONE | crates/base/src/sound.rs |
| `method PlayAsync` | DONE | crates/base/src/sound.rs |

## `SplashScreenForm.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class SplashScreenForm` | DONE | app/src/forms/splash.rs |
| `field mVersion` | DONE | app/src/forms/splash.rs |
| `ctor SplashScreenForm` | DONE | app/src/forms/splash.rs |
| `method OnPaint` | DONE | app/src/forms/splash.rs |
| `method SplashScreenForm_Load` | DONE | app/src/forms/splash.rs |
| `method timer1_Tick` | DONE | app/src/forms/splash.rs |
| `method pictureBox1_Click` | DONE | app/src/forms/splash.rs |
| `method CloseAndGoMain` | DONE | app/src/forms/splash.rs |

## `StateBuilder.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class JogCommand` | DONE | crates/gcode/src/command.rs `GrblCommand::new_jog` + `is_jog` flag |
| `ctor JogCommand` | DONE | crates/gcode/src/command.rs `GrblCommand::new_jog` + `is_jog` flag |
| `class GrblCommand` | DONE | crates/gcode/src/state_builder.rs |
| `class StatePositionBuilder` | DONE | crates/gcode/src/state_builder.rs |
| `class CumulativeElement` | DONE | crates/gcode/src/state_builder.rs |
| `ctor CumulativeElement` | DONE | crates/gcode/src/state_builder.rs |
| `method Update` | DONE | crates/gcode/src/state_builder.rs |
| `class LastValueElement` | DONE | crates/gcode/src/state_builder.rs |
| `ctor LastValueElement` | DONE | crates/gcode/src/state_builder.rs |
| `method Update` | DONE | crates/gcode/src/state_builder.rs |
| `field mCurX` | DONE | crates/gcode/src/state_builder.rs |
| `field mCurY` | DONE | crates/gcode/src/state_builder.rs |
| `field mCurZ` | DONE | crates/gcode/src/state_builder.rs |
| `field mWcoX` | DONE | crates/gcode/src/state_builder.rs |
| `field mWcoY` | DONE | crates/gcode/src/state_builder.rs |
| `field mWcoZ` | DONE | crates/gcode/src/state_builder.rs |
| `field mCurF` | DONE | crates/gcode/src/state_builder.rs |
| `field mCurS` | DONE | crates/gcode/src/state_builder.rs |
| `field LastArcHelperResult` | DONE | crates/gcode/src/state_builder.rs |
| `method AnalyzeCommand` | DONE | crates/gcode/src/state_builder.rs |
| `method UpdateFS` | DONE | crates/gcode/src/state_builder.rs |
| `method UpdateXYZ` | DONE | crates/gcode/src/state_builder.rs |
| `method UpdateWCO` | DONE | crates/gcode/src/state_builder.rs |
| `method TrueMovement` | DONE | crates/gcode/src/state_builder.rs |
| `method ComputeExecutionTime` | DONE | crates/gcode/src/state_builder.rs |
| `method GetSegmentLenght` | DONE | crates/gcode/src/state_builder.rs |
| `method GetCurrentAlpha` | DONE | crates/gcode/src/state_builder.rs |
| `method Homing` | DONE | crates/gcode/src/state_builder.rs |
| `method GetArcHelper` | DONE | crates/gcode/src/state_builder.rs |
| `class StateBuilder` | DONE | crates/gcode/src/state_builder.rs |
| `class ModalElement` | DONE | crates/gcode/src/state_builder.rs |
| `ctor ModalElement` | DONE | crates/gcode/src/state_builder.rs |
| `method Update` | DONE | crates/gcode/src/state_builder.rs |
| `field MotionMode` | DONE | crates/gcode/src/state_builder.rs |
| `field CoordinateSelect` | DONE | crates/gcode/src/state_builder.rs |
| `field PlaneSelect` | DONE | crates/gcode/src/state_builder.rs |
| `field DistanceMode` | DONE | crates/gcode/src/state_builder.rs |
| `field ArcDistanceMode` | DONE | crates/gcode/src/state_builder.rs |
| `field FeedRateMode` | DONE | crates/gcode/src/state_builder.rs |
| `field UnitsMode` | DONE | crates/gcode/src/state_builder.rs |
| `field CutterRadiusCompensation` | DONE | crates/gcode/src/state_builder.rs |
| `field ToolLengthOffset` | DONE | crates/gcode/src/state_builder.rs |
| `field ProgramMode` | DONE | crates/gcode/src/state_builder.rs |
| `field CoolantState` | DONE | crates/gcode/src/state_builder.rs |
| `field SpindleState` | DONE | crates/gcode/src/state_builder.rs |
| `method UpdateModalsNB` | DONE | crates/gcode/src/state_builder.rs |
| `method GetSettledModals` | DONE | crates/gcode/src/state_builder.rs |
| `method AddSettled` | DONE | crates/gcode/src/state_builder.rs |
| `class G2G3Helper` | DONE | crates/gcode/src/state_builder.rs |
| `field CenterX` | DONE | crates/gcode/src/state_builder.rs |
| `field CenterY` | DONE | crates/gcode/src/state_builder.rs |
| `property Lenght` | DONE | crates/gcode/src/state_builder.rs |
| `property AbsLenght` | DONE | crates/gcode/src/state_builder.rs |
| `field CW` | DONE | crates/gcode/src/state_builder.rs |
| `field Ray` | DONE | crates/gcode/src/state_builder.rs |
| `field RectX` | DONE | crates/gcode/src/state_builder.rs |
| `field RectY` | DONE | crates/gcode/src/state_builder.rs |
| `field RectW` | DONE | crates/gcode/src/state_builder.rs |
| `field RectH` | DONE | crates/gcode/src/state_builder.rs |
| `field BBox` | DONE | crates/gcode/src/state_builder.rs |
| `field StartAngle` | DONE | crates/gcode/src/state_builder.rs |
| `field EndAngle` | DONE | crates/gcode/src/state_builder.rs |
| `field AngularWidth` | DONE | crates/gcode/src/state_builder.rs |
| `ctor G2G3Helper` | DONE | crates/gcode/src/state_builder.rs |
| `method Circle` | DONE | crates/gcode/src/state_builder.rs |
| `method CalculateAngle` | DONE | crates/gcode/src/state_builder.rs |
| `method AngularDistance` | DONE | crates/gcode/src/state_builder.rs |
| `field a0` | DONE | crates/gcode/src/state_builder.rs |
| `field a90` | DONE | crates/gcode/src/state_builder.rs |
| `field a180` | DONE | crates/gcode/src/state_builder.rs |
| `field a270` | DONE | crates/gcode/src/state_builder.rs |
| `field a360` | DONE | crates/gcode/src/state_builder.rs |
| `method GetQuadrant` | DONE | crates/gcode/src/state_builder.rs |
| `method BBBox` | DONE | crates/gcode/src/state_builder.rs |

## `SvgConverter/BezierTools.cs` (WP-E)

| member | status | Rust location / reason |
|---|---|---|
| `class BezierTools` | DONE | `crates/svg2gcode/src/bezier.rs` (`flatten_to`, private helpers) |
| `method SplitCurveAtT` | DONE | `crates/svg2gcode/src/bezier.rs` (`flatten_to`, private helpers) |
| `method FlattenSegmentTo` | DONE | `crates/svg2gcode/src/bezier.rs` (`flatten_to`, private helpers) |
| `method CalculateTriangleArea` | DONE | `crates/svg2gcode/src/bezier.rs` (`flatten_to`, private helpers) |
| `method FlattenTo` | DONE | `crates/svg2gcode/src/bezier.rs` (`flatten_to`, private helpers) |

## `SvgConverter/ConvertSizeAndOptionForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class SvgToGCodeForm` | DONE | app/src/forms/params_dock.rs |
| `field LaserOptions` | DONE | app/src/forms/params_dock.rs |
| `field FilterOptions` | DONE | app/src/forms/params_dock.rs |
| `method ColorFilterToItem` | DONE | app/src/forms/params_dock.rs |
| `class ComboboxItem` | DONE | app/src/forms/params_dock.rs |
| `property Text` | DONE | app/src/forms/params_dock.rs |
| `property Value` | DONE | app/src/forms/params_dock.rs |
| `ctor ComboboxItem` | DONE | app/src/forms/params_dock.rs |
| `method ToString` | DONE | app/src/forms/params_dock.rs |
| `method CreateAndShowDialog` | DONE | app/src/forms/params_dock.rs |
| `ctor SvgToGCodeForm` | DONE | app/src/forms/params_dock.rs |
| `method AssignMinMaxLimit` | DONE | app/src/forms/params_dock.rs |
| `method ShowDialogForm` | DONE | app/src/forms/params_dock.rs |
| `method RefreshPerc` | DONE | app/src/forms/params_dock.rs |
| `method BtnOnOffInfo_Click` | DONE | app/src/forms/params_dock.rs |
| `method BtnModulationInfo_Click` | DONE | app/src/forms/params_dock.rs |
| `method CBLaserON_SelectedIndexChanged` | DONE | app/src/forms/params_dock.rs |
| `method BtnPSHelper_Click` | DONE | app/src/forms/params_dock.rs |
| `method BtnColorFilter_Click` | DONE | app/src/forms/params_dock.rs |

## `SvgConverter/GCodeFromSVG.cs` (WP-E)

| member | status | Rust location / reason |
|---|---|---|
| `enum ColorFilter` | DONE | `convert.rs::ColorFilter` |
| `field factor_In2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field factor_Mm2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field factor_Cm2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field factor_Pt2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field factor_Pc2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field factor_Em2Px` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field gcodeString` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgBezierAccuracy` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgScaleApply` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgMaxSize` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgClosePathExtend` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgNodesOnly` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgPauseElement` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgPausePenDown` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgComments` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field gcodeReduce` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field gcodeReduceVal` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field UserOffset` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field GCodeXYFeed` | DONE | `GCodeFromSvg::gcode_xy_feed` (NOTE(upstream): never read; feed comes from option `border_speed`) |
| `field scaledError` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgConvertToMM` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field gcodeScale` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field matrixGroup` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field matrixElement` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field oldMatrixElement` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field loadColorFilter` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field FilterColorLimitHLow` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field FilterColorLimitHigh` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field SvgStrokeAttribute` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field SvgFillAttribute` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgCode` | DONE | argument of `convert_svg` (roxmltree `Node`; document is local to `convert_from_text`) |
| `field importInMM` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method convertFromText` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method convertFromFile` | DONE | `convert.rs::GCodeFromSvg::convert_from_file` (embedded resources: caller reads text, calls `convert_from_text`) |
| `method convertSVG` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method startConvert` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field nspace` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseGlobals` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseGroup` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseTransform` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method ParseTransform` | DONE | `convert.rs::parse_transform_string` (pub) |
| `method GetTextBetween` | DONE | `convert.rs::get_text_between` (pub) |
| `method ConvertToPixel` | DONE | `convert.rs::convert_to_pixel` (pub) |
| `method getColor` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseBasicElements` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parsePath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method filterByColor` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseRgb` | DONE | `crates/svg2gcode/src/color.rs::from_html` (ColorTranslator.FromHtml, Mono rules + HTML system colours) |
| `method parseStyle` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parseColorAttributes` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field penIsDown` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field startSubPath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field countSubPath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field startPath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field startFirstElement` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field svgHeightPx` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field offsetY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field currentY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field firstY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field lastY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field cxMirror` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field secondMove` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method parsePathCommand` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method calcArc` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method CalculateVectorAngle` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method fsqrt` | DONE | `crates/svg2gcode/src/gcode.rs::fsqrt` (shared helper; unused in converter itself) |
| `method fvmag` | DONE | `gcode.rs::fvmag` |
| `method fdistance` | DONE | `gcode.rs::fdistance` |
| `method GetBezierApproximation` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method GetBezierApproximationOld` | DONE | `convert.rs::bezier_approximation_old` (pub) |
| `method GetBezierPoint` | DONE | `convert.rs::get_bezier_point` / `bezier_approximation_old` (pub) |
| `method translateXY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method translateXY` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method translateIJ` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method translateIJ` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeDotOnly` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeStartPath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeStopPath` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeMoveTo` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field isReduceOk` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field rejectPoint` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `field lastGCX` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `property UseLegacyBezier` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `property SvgScaleApply` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `property SvgMaxSize` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeMoveTo` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodeArcToCCW` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodePenUp` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |
| `method gcodePenDown` | DONE | `crates/svg2gcode/src/convert.rs` (`GCodeFromSvg`) |

## `SvgConverter/SvgModeForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class SvgModeForm` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `property Result` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `enum Mode` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `ctor SvgModeForm` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `method CreateAndShow` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `method CreatePreview` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `method PbVector_Click` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |
| `method PbImage_Click` | DONE | app/src/forms/svg_mode.rs (unused in C# flow (mode forced to Vector); both previews use one resvg render) |

## `SvgConverter/gcodeRelated.cs` (WP-E)

| member | status | Rust location / reason |
|---|---|---|
| `class gcode` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field formatCode` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field formatNumber` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeDragCompensation` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeDragRadius` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeDragAngle` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeXYFeed` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeSpindleSpeed` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeSpindleCmdOn` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeSpindleCmdOff` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeCompress` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeNoArcs` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field gcodeAngleStep` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field mDecimalPlaces` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field firmwareType` | DONE | `SvgConvOptions::firmware` (caller reads setting once per process) |
| `field rapidnum` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field SupportPWM` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method setup` | DONE | `Gcode::setup(&SvgConvOptions)` (replaces `Settings.GetObject` + `GrblCore.Configuration.MaxPWM`) |
| `method setRapidNum` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method setDecimalPlaces` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method getIntGCode` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method getStrGCode` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method getIntValue` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method getStringValue` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method frmtCode` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method frmtNum` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method frmtNum` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field secondMove` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field applyXYFeedRate` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method SpindleOn` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method SpindleOff` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method PutInitialCommand` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method PutFinalCommand` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method PenDown` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method PenUp` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field lastg` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field lastMovewasG0` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveTo` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveTo` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveTo` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveToRapid` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveToRapid` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveSplit` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field remainingC` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field segFinalX` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveSplit` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field dragArc` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field origFinalY` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `field dragCompi` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method dragToolCompensation` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method Move` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method Move` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method splitLine` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method Arc` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method Arc` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method MoveArc` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method splitArc` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method getAngle` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method fsqrt` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method fvmag` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |
| `method fdistance` | DONE | `crates/svg2gcode/src/gcode.rs` (`Gcode`, state of the static class) |

## `SvgIcons/SvgIcons.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class SvgIcons` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |
| `field mIconList` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |
| `ctor SvgIcons` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |
| `method LoadSvg` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |
| `method LoadImage` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |
| `method Contains` | DONE | `ui/icons.rs` (`Icons::new(false)`, `render_svg` via resvg) |

## `Telegram.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class Telegram` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `method NotifyEvent` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `method NotifyEvent` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `method InternalNotifyEvent` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `class MyWebClient` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |
| `method GetWebRequest` | N/A | removed: no telemetry / phone-home (Telegram notifier deleted) |

## `Tools/Application.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class Application2` | N/A | WinForms application helper (restart); replaced by MainForm restart_on_exit in app/src/main_form.rs |
| `method RestartNoCommandLine` | N/A | WinForms application helper (restart); replaced by MainForm restart_on_exit in app/src/main_form.rs |

## `Tools/Cronometro.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class TimingBase` | DONE | crates/base/src/tools/cronometro.rs |
| `field mStartup` | DONE | crates/base/src/tools/cronometro.rs |
| `ctor TimingBase` | DONE | crates/base/src/tools/cronometro.rs |
| `method TimeFromApplicationStartup` | DONE | crates/base/src/tools/cronometro.rs |
| `class ElapsedFromEvent` | DONE | crates/base/src/tools/cronometro.rs |
| `field started` | DONE | crates/base/src/tools/cronometro.rs |
| `field starttime` | DONE | crates/base/src/tools/cronometro.rs |
| `method Start` | DONE | crates/base/src/tools/cronometro.rs |
| `method Reset` | DONE | crates/base/src/tools/cronometro.rs |
| `class SimpleCrono` | DONE | crates/base/src/tools/cronometro.rs |
| `field timer` | DONE | crates/base/src/tools/cronometro.rs |
| `field elapsed` | DONE | crates/base/src/tools/cronometro.rs |
| `ctor SimpleCrono` | DONE | crates/base/src/tools/cronometro.rs |
| `method Start` | DONE | crates/base/src/tools/cronometro.rs |
| `method Stop` | DONE | crates/base/src/tools/cronometro.rs |
| `method Reset` | DONE | crates/base/src/tools/cronometro.rs |
| `class AdvancedCrono` | DONE | crates/base/src/tools/cronometro.rs |
| `field times` | DONE | crates/base/src/tools/cronometro.rs |
| `method SalvaIntermedio` | DONE | crates/base/src/tools/cronometro.rs |
| `method Reset` | DONE | crates/base/src/tools/cronometro.rs |
| `class PeriodicEventTimer` | DONE | crates/base/src/tools/cronometro.rs |
| `field m_period` | DONE | crates/base/src/tools/cronometro.rs |
| `field crono` | DONE | crates/base/src/tools/cronometro.rs |
| `field m_periodcount` | DONE | crates/base/src/tools/cronometro.rs |
| `ctor PeriodicEventTimer` | DONE | crates/base/src/tools/cronometro.rs |
| `method ReInit` | DONE | crates/base/src/tools/cronometro.rs |
| `method Start` | DONE | crates/base/src/tools/cronometro.rs |

## `Tools/DataProtector.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class Protector` | N/A | only protected the Telegram bot code; Telegram removed |
| `field Scope` | N/A | only protected the Telegram bot code; Telegram removed |
| `method Encrypt` | N/A | only protected the Telegram bot code; Telegram removed |
| `method Decrypt` | N/A | only protected the Telegram bot code; Telegram removed |

## `Tools/DoubleBufferBmp.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class DoubleBufferBmp` | N/A | GDI double buffering, egui paints retained-free |
| `field mBmps` | N/A | GDI double buffering, egui paints retained-free |
| `field mGetIdx` | N/A | GDI double buffering, egui paints retained-free |

## `Tools/ExpressionEvaluator.cs` (WP-D)

| member | status | Rust location / reason |
|---|---|---|
| `class Expression` | DONE | `lg_expr::Expression` (crates/expr/src/lib.rs) |
| `class DynamicFunction` | DONE | `Expression::dynamic_function` holds the emulated compiled program (`Op`) instead of an emitted .NET class |
| `method EvaluateD` | DONE | `Expression::evaluate_d` / `run_compiled` |
| `method EvaluateB` | DONE | `Expression::evaluate_b` |
| `field dynamicFunction` | DONE | `Expression::dynamic_function` |
| `field inFunction` | DONE | `Expression::in_function` |
| `field postFunction` | DONE | `Expression::post_function` |
| `field variables` | DONE | `Expression::variables` (insertion-ordered Vec) |
| `field TRUE` | DONE | `const TRUE` |
| `field FALSE` | DONE | `const FALSE` |
| `field splitPostFunction` | DONE | `Expression::split_post_function` |
| `field compilecode` | DONE | `Expression::compilecode`, `compile_enabled`/`set_compile_enabled` |
| `ctor Expression` | DONE | `Expression::default()` / `Expression::new(text)` |
| `ctor Expression` | DONE | `Expression::default()` / `Expression::new(text)` |
| `method Infix2Postfix` | DONE | `infix2postfix` (private) |
| `method IsOperand` | DONE | `lg_expr::is_operand` |
| `method IsOperator` | DONE | `lg_expr::is_operator` |
| `method Expand` | DONE | `lg_expr::expand`, `Expression::expand` (crates/expr/src/expand.rs; regexes hand-coded) |
| `method ScientificNotationCorrection` | DONE | `lg_expr::scientific_notation_correction` |
| `method GetPrecedence` | DONE | `lg_expr::get_precedence` |
| `method AddSetVariable` | DONE | `Expression::add_set_variable` / `add_set_variable_bool` |
| `method AddSetVariable` | DONE | `Expression::add_set_variable` / `add_set_variable_bool` |
| `method ClearVariables` | DONE | `Expression::clear_variables` |
| `method Clear` | DONE | `Expression::clear` |
| `method IsVariable` | DONE | `lg_expr::is_variable` |
| `method GetVariableValue` | DONE | `Expression::get_variable_value` |
| `method ExistToken` | DONE | `Expression::exist_token` |
| `method Validate` | DONE | `validate` (private) |
| `method EvaluateD` | DONE | `Expression::evaluate_d` / `run_compiled` |
| `method ConvertString` | DONE | `Expression::convert_string` (private) |
| `method ToString` | DONE | `impl Display for Expression` |
| `method EvaluateB` | DONE | `Expression::evaluate_b` |
| `method IsNumber` | DONE | `lg_expr::is_number` |
| `method compile` | DONE | `Expression::compile` builds an `Op` program (IL emulation, not golden-tested: Mono emits invalid IL for EvaluateB) |
| `method emitFunction` | DONE | folded into `Expression::compile` / `run_compiled` |

## `Tools/FormsHelper.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class FormsHelper` | N/A | WinForms helper (MainForm lookup, invoke); egui single-thread UI needs none |
| `property MainForm` | N/A | WinForms helper (MainForm lookup, invoke); egui single-thread UI needs none |

## `Tools/Graph.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class Graph` | DONE | `app/src/ui/widgets/graph.rs` (`rounded_rect_points`, `fill_rounded`) |
| `method RoundedRect` | DONE | `app/src/ui/widgets/graph.rs` (`rounded_rect_points`, `fill_rounded`) |

## `Tools/HiResTimer.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class HiResTimer` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field ErrorThreshold` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field MILLI_IN_SECOND` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field NANO_IN_MILLI` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field NANO_IN_SECOND` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field TimingLock` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field mPerfCounterSupported` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mCurrentFrequency` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field mOriginalFrequency` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field mLastReference` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field mTotalNano` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `field LongWindow` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mCorrectionCount` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field startDT` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field startQPC` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field startEPC` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field testmultiplier` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method QueryPerformanceCounter` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method QueryPerformanceFrequency` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method GetTickCount` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mTickCount64` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method GetTickCount64` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `struct TimeReference` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mLowRes` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mHiRes` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `field mReference` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `property ReferenceNano` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `ctor TimeReference` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `ctor TimeReference` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method Subtract` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method Subtract` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `property LowRes` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `property LowResNano` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `property HiRes` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `property HiResNano` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method CalculateHiResFrequency` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method IsGoodHiRes` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method ShouldAdjustFreqency` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `ctor HiResTimer` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `property TotalMilliseconds` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `property CurrentFrequency` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `property OriginalFrequency` | DONE | crates/base/src/tools/hires_timer.rs (Utils: crates/base/src/tools/utils.rs) |
| `method ComputeFrequency` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `method ApplicaNuovaFrequenza` | N/A | QPC / GetTickCount low-res frequency correction not needed: Rust uses monotonic Instant (ns), sleep/hibernation guard kept in tools/hires_timer.rs |
| `class Utils` | DONE | crates/base/src/tools/utils.rs |
| `method TimeSpanToString` | DONE | crates/base/src/tools/utils.rs |
| `method HumanReadableDateDiff` | DONE | crates/base/src/tools/utils.rs |
| `method OpenLink` | DONE | crates/base/src/tools/utils.rs |
| `enum TimePrecision` | DONE | crates/base/src/tools/utils.rs |

## `Tools/InputBox.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class InputBox` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field buttonOK` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field buttonCancel` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field labelPrompt` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field textBoxText` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field errorProviderText` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field components` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field _validator` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `ctor InputBox` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method Dispose` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method buttonCancel_Click` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method buttonOK_Click` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method Show` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method Show` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method textBoxText_TextChanged` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `method textBoxText_Validating` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `class InputBoxResult` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field OK` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field Text` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `class InputBoxValidatingArgs` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field Text` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field Message` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `field Cancel` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |
| `delegate void` | DONE | `app/src/ui/widgets/input_box.rs` (`InputBox`, `InputBoxResult`) |

## `Tools/Json.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class JSONParser` | N/A | removed: no update check / phone-home |
| `field splitArrayPool` | N/A | allocation cache |
| `field stringBuilder` | N/A | allocation cache |
| `field fieldInfoCache` | N/A | reflection cache; typed parse only for OnlineVersion |
| `field propertyInfoCache` | N/A | reflection cache; typed parse only for OnlineVersion |
| `method AppendUntilStringEnd` | N/A | removed: no update check / phone-home |
| `method Split` | N/A | removed: no update check / phone-home |
| `method ParseValue` | N/A | removed: no update check / phone-home |
| `method ParseAnonymousValue` | N/A | removed: no update check / phone-home |
| `method ParseObject` | N/A | removed: no update check / phone-home |

## `Tools/MathHelper.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class MathHelper` | DONE | crates/base/src/tools/math_helper.rs |
| `method LinearDistance` | DONE | crates/base/src/tools/math_helper.rs |
| `class RulerStepCalculator` | DONE | crates/base/src/tools/math_helper.rs |
| `field mtable` | DONE | crates/base/src/tools/math_helper.rs |
| `field stable` | DONE | crates/base/src/tools/math_helper.rs |
| `field _bigstep` | DONE | crates/base/src/tools/math_helper.rs |
| `field _smallstep` | DONE | crates/base/src/tools/math_helper.rs |
| `field _firstsmall` | DONE | crates/base/src/tools/math_helper.rs |
| `field _firstbig` | DONE | crates/base/src/tools/math_helper.rs |
| `field _min` | DONE | crates/base/src/tools/math_helper.rs |
| `field _max` | DONE | crates/base/src/tools/math_helper.rs |
| `field _steps` | DONE | crates/base/src/tools/math_helper.rs |
| `ctor RulerStepCalculator` | DONE | crates/base/src/tools/math_helper.rs |

## `Tools/MobileAverageCalculator.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class MobileAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mSize` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mFilledSize` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mSum` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mCur` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mMax` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mMin` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mAvg` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mReady` | DONE | crates/base/src/tools/mobile_average.rs |
| `ctor MobileAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `ctor MobileAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `method Reset` | DONE | crates/base/src/tools/mobile_average.rs |
| `method EnqueueNewSample` | DONE | crates/base/src/tools/mobile_average.rs |
| `method Resize` | DONE | crates/base/src/tools/mobile_average.rs |
| `class MobileDAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mSize` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mSum` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mCur` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mMax` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mMin` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mAvg` | DONE | crates/base/src/tools/mobile_average.rs |
| `field mReady` | DONE | crates/base/src/tools/mobile_average.rs |
| `ctor MobileDAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `ctor MobileDAverageCalculator` | DONE | crates/base/src/tools/mobile_average.rs |
| `method Reset` | DONE | crates/base/src/tools/mobile_average.rs |
| `method Reset` | DONE | crates/base/src/tools/mobile_average.rs |
| `method EnqueueNewSample` | DONE | crates/base/src/tools/mobile_average.rs |
| `method Resize` | DONE | crates/base/src/tools/mobile_average.rs |

## `Tools/OSHelper.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class OSHelper` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field Is64BitProcess` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field Is64BitOperatingSystem` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetClrInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetOSInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetBitFlag` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method SetBit` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `class OSVersionInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `enum SoftwareArchitecture` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `enum ProcessorArchitecture` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `delegate bool` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field s_Edition` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field s_Name` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method IsWindows11` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetProductInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetVersionEx` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetSystemMetrics` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetSystemInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetNativeSystemInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `struct OSVERSIONINFOEX` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwOSVersionInfoSize` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwMajorVersion` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwMinorVersion` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwBuildNumber` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwPlatformId` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field szCSDVersion` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wServicePackMajor` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wServicePackMinor` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wSuiteMask` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wProductType` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wReserved` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `struct SYSTEM_INFO` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field uProcessorInfo` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwPageSize` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field lpMinimumApplicationAddress` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field lpMaximumApplicationAddress` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwActiveProcessorMask` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwNumberOfProcessors` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwProcessorType` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwAllocationGranularity` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwProcessorLevel` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwProcessorRevision` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `struct _PROCESSOR_INFO_UNION` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field dwOemId` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wProcessorArchitecture` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field wReserved` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method LoadLibrary` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetProcAddress` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field pdic` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_NT_WORKSTATION` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_NT_DOMAIN_CONTROLLER` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_NT_SERVER` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_SMALLBUSINESS` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_ENTERPRISE` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_TERMINAL` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_DATACENTER` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_SINGLEUSERTS` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_PERSONAL` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `field VER_SUITE_BLADE` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method GetIsWow64ProcessDelegate` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method Is32BitProcessOn64BitProcessor` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method IsWindows10` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |
| `method RegistryRead` | N/A | Windows version probing; lg_base::logger::os_info used for logs and exception report |

## `Tools/Project.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class Project` | DONE | crates/services/src/project.rs |
| `field Formatter` | DONE | crates/services/src/project.rs |
| `field ProjectSettings` | DONE | crates/services/src/project.rs |
| `method AddSettings` | DONE | crates/services/src/project.rs |
| `method ClearSettings` | DONE | crates/services/src/project.rs |
| `method StoreSettings` | DONE | crates/services/src/project.rs |
| `method LoadProject` | DONE | crates/services/src/project.rs |
| `method ConvertImageToBase64` | DONE | crates/services/src/project.rs |
| `method SaveImage` | DONE | crates/services/src/project.rs |

## `Tools/RetainedSetting.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class RetainedSetting` | DONE | crates/base/src/tools/retained_setting.rs |
| `field OnChange` | DONE | crates/base/src/tools/retained_setting.rs |
| `field mSettingName` | DONE | crates/base/src/tools/retained_setting.rs |
| `field mValue` | DONE | crates/base/src/tools/retained_setting.rs |
| `ctor RetainedSetting` | DONE | crates/base/src/tools/retained_setting.rs |

## `Tools/Serializer.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class Serializer` | DONE | crates/base/src/nrbf/serializer.rs |
| `field DataStartString` | DONE | crates/base/src/nrbf/serializer.rs |
| `field ThreadLock` | DONE | crates/base/src/nrbf/serializer.rs `THREAD_LOCK` (P4-2: added; also keeps the tmp file name unique) |
| `field SerializerVersion` | DONE | crates/base/src/nrbf/serializer.rs |
| `class TAGS` | DONE | crates/base/src/nrbf/serializer.rs |
| `field SerializerVersionTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `field EncriptionTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `field CompressionTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `field SerializationModeTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `field InitializationVectorTag` | N/A | password mode not ported (no cipher dependency); the only C# caller with a password is the UsageStats news-message file (`"lasergrbl"`), remote telemetry not ported (PLAN section 6) |
| `field PwdHashTag` | N/A | password mode not ported |
| `method ObjToFile` | DONE | crates/base/src/nrbf/serializer.rs (obj_to_file/obj_to_bytes; password overloads N/A) |
| `method ObjToFile` | DONE | crates/base/src/nrbf/serializer.rs (obj_to_file/obj_to_bytes; password overloads N/A) |
| `method ObjToFile` | DONE | crates/base/src/nrbf/serializer.rs (obj_to_file/obj_to_bytes; password overloads N/A) |
| `method ObjToFile` | DONE | crates/base/src/nrbf/serializer.rs (obj_to_file/obj_to_bytes; password overloads N/A) |
| `method ObjToFile` | DONE | crates/base/src/nrbf/serializer.rs (obj_to_file/obj_to_bytes; password overloads N/A) |
| `method ObjFromFile` | DONE | crates/base/src/nrbf/serializer.rs |
| `method ObjFromFile` | DONE | crates/base/src/nrbf/serializer.rs |
| `method ObjFromFile` | DONE | crates/base/src/nrbf/serializer.rs |
| `enum SerializationMode` | N/A | only Binary exists in practice |
| `method CreateFormatterForMode` | N/A | C# always returns BinaryFormatter; Rust always NRBF |
| `method WriteSerializerTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `method WriteTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `method WriteEndTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `method GetSerializerTag` | DONE | crates/base/src/nrbf/serializer.rs |
| `field RegX` | DONE | crates/base/src/nrbf/serializer.rs |
| `method GetTagValue` | DONE | crates/base/src/nrbf/serializer.rs |
| `method GenerateKey` | N/A | password mode not ported |
| `method GenerateHash` | N/A | password mode not ported |
| `method ConvertStringToASCIIByte` | DONE | crates/base/src/nrbf/serializer.rs |
| `method ModeFromFname` | N/A | always Binary |
| `method ManageOrphanTmp` | DONE | crates/base/src/nrbf/serializer.rs |
| `method ManageWriteError` | DONE | crates/base/src/nrbf/serializer.rs |
| `method ManageReadError` | DONE | crates/base/src/nrbf/serializer.rs |
| `method LogFileUsage` | N/A | empty in C# |
| `class LogDatedWriter` | N/A | C# only forwards to Logger.LogMessage; Rust calls logger directly |
| `ctor LogDatedWriter` | N/A | see LogDatedWriter |
| `method WriteLogLines` | N/A | see LogDatedWriter |
| `method WriteEmptyLine` | N/A | see LogDatedWriter |
| `method WriteSeparator` | N/A | see LogDatedWriter |
| `method Close` | N/A | see LogDatedWriter |
| `class MissingPasswordException` | DONE | crates/base/src/nrbf/serializer.rs (SerializerError::MissingPassword) |
| `ctor MissingPasswordException` | DONE | crates/base/src/nrbf/serializer.rs (SerializerError::MissingPassword) |
| `class WrongPasswordException` | N/A | password mode not ported |
| `ctor WrongPasswordException` | N/A | password mode not ported |
| `method ManageOldVersion` | DONE | crates/base/src/nrbf/serializer.rs (obj_from_bytes: headerless NRBF accepted; no re-save) |

## `Tools/TaskScheduler.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class TaskScheduler` | N/A | NtSetTimerResolution is Windows only (PLAN section 6) |
| `method NtSetTimerResolution` | N/A | NtSetTimerResolution is Windows only (PLAN section 6) |
| `method SetClockResolution` | N/A | NtSetTimerResolution is Windows only (PLAN section 6) |

## `Tools/ThreadClass.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class ThreadObject` | DONE | crates/base/src/tools/thread_class.rs |
| `field _delegateSub` | DONE | crates/base/src/tools/thread_class.rs |
| `field _firstRunSub` | DONE | crates/base/src/tools/thread_class.rs |
| `ctor ThreadObject` | DONE | crates/base/src/tools/thread_class.rs |
| `method OnFirstRun` | DONE | crates/base/src/tools/thread_class.rs |
| `method DoTheWork` | DONE | crates/base/src/tools/thread_class.rs |
| `class ThreadClass` | DONE | crates/base/src/tools/thread_class.rs |
| `field MustExit` | DONE | crates/base/src/tools/thread_class.rs |
| `field UserHandle` | DONE | crates/base/src/tools/thread_class.rs |
| `field TH` | DONE | crates/base/src/tools/thread_class.rs |
| `field mApartment` | N/A | COM apartment is Windows-only |
| `field mPriority` | N/A | thread priority not set (std has no portable API) |
| `field mHandleArray` | DONE | crates/base/src/tools/thread_class.rs |
| `ctor ThreadClass` | DONE | crates/base/src/tools/thread_class.rs |
| `method MustRun` | DONE | crates/base/src/tools/thread_class.rs |
| `method OnThreadTerminating` | DONE | crates/base/src/tools/thread_class.rs |
| `method Loop` | DONE | crates/base/src/tools/thread_class.rs |
| `method OnFirstRun` | DONE | crates/base/src/tools/thread_class.rs |
| `method DoTheWork` | DONE | crates/base/src/tools/thread_class.rs |
| `method Start` | DONE | crates/base/src/tools/thread_class.rs |
| `property Running` | DONE | crates/base/src/tools/thread_class.rs |
| `field _Name` | DONE | crates/base/src/tools/thread_class.rs |
| `property Name` | DONE | crates/base/src/tools/thread_class.rs |
| `field _timeout` | DONE | crates/base/src/tools/thread_class.rs |
| `property StopWaitTimeout` | DONE | crates/base/src/tools/thread_class.rs |
| `field _sleeptime` | DONE | crates/base/src/tools/thread_class.rs |
| `property SleepTime` | DONE | crates/base/src/tools/thread_class.rs |
| `method Stop` | DONE | crates/base/src/tools/thread_class.rs |
| `method AutoDispose` | N/A | Rust Drop stops the thread |
| `method MustExitTH` | DONE | crates/base/src/tools/thread_class.rs |
| `method MustExitTH` | DONE | crates/base/src/tools/thread_class.rs |
| `method Dispose` | DONE | crates/base/src/tools/thread_class.rs |

## `Tools/WinAPI.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class ModifyProgressBarColor` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `method SendMessage` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `method SetState` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `method SetThreadExecutionState` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `enum EXECUTION_STATE` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `method SignalActvity` | N/A | Windows sleep prevention; not ported (PLAN section 6) |
| `method SignalFree` | N/A | Windows sleep prevention; not ported (PLAN section 6) |

## `Tools/WordWrap.cs` (WP-C1)

| member | status | Rust location / reason |
|---|---|---|
| `class WordWrap` | DONE | crates/base/src/tools/word_wrap.rs |
| `field _newline` | DONE | crates/base/src/tools/word_wrap.rs |
| `method WrapString` | DONE | crates/base/src/tools/word_wrap.rs |
| `method BreakLine` | DONE | crates/base/src/tools/word_wrap.rs |

## `UrlManager.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class UrlManager` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field UpdateMain` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field UpdateMirror` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field Statistics` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field TelegramHandler` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field TelegramServiceKey` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field UpdateMaterialDB` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field DownloadMaterialDB` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |
| `field LaserStatistics` | N/A | removed: only held phone-home endpoints (update, statistics, Telegram, material DB download) |

## `UsageStats.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class UsageStats` | DONE | crates/core/src/usage_stats.rs |
| `field Firmware` | DONE | crates/core/src/usage_stats.rs |
| `class UsageCounters` | DONE | crates/core/src/usage_stats.rs |
| `field GCodeFile` | DONE | crates/core/src/usage_stats.rs |
| `field RasterFile` | DONE | crates/core/src/usage_stats.rs |
| `field Vectorization` | DONE | crates/core/src/usage_stats.rs |
| `field Centerline` | DONE | crates/core/src/usage_stats.rs |
| `field Dithering` | DONE | crates/core/src/usage_stats.rs |
| `field Line2Line` | DONE | crates/core/src/usage_stats.rs |
| `field SvgFile` | DONE | crates/core/src/usage_stats.rs |
| `field Passthrough` | DONE | crates/core/src/usage_stats.rs |
| `method Update` | DONE | crates/core/src/usage_stats.rs |
| `field InstallationID` | DONE | crates/core/src/usage_stats.rs |
| `field LastSent` | DONE | crates/core/src/usage_stats.rs |
| `field InstalledDate` | DONE | crates/core/src/usage_stats.rs |
| `field Version` | DONE | crates/core/src/usage_stats.rs |
| `field GrblVersion` | DONE | crates/core/src/usage_stats.rs |
| `field Locale` | DONE | crates/core/src/usage_stats.rs |
| `field UiLang` | DONE | crates/core/src/usage_stats.rs |
| `field UsageCount` | DONE | crates/core/src/usage_stats.rs |
| `field UsageTime` | DONE | crates/core/src/usage_stats.rs |
| `field Wrapper` | DONE | crates/core/src/usage_stats.rs |
| `field FirmwareString` | DONE | crates/core/src/usage_stats.rs |
| `field VendorString` | DONE | crates/core/src/usage_stats.rs |
| `field Counters` | DONE | crates/core/src/usage_stats.rs |
| `field mManager` | DONE | crates/core/src/usage_stats.rs |
| `field data` | DONE | crates/core/src/usage_stats.rs |
| `field datafilename` | DONE | crates/core/src/usage_stats.rs |
| `field messagefilename` | DONE | crates/core/src/usage_stats.rs |
| `property Messages` | N/A | server news messages, PLAN 6 |
| `field DoNotSendNow` | DONE | crates/core/src/usage_stats.rs |
| `method LoadFile` | DONE | crates/core/src/usage_stats.rs |
| `method FixFile` | N/A | BinaryFormatter migration, not needed |
| `method FixFile360` | N/A | BinaryFormatter migration, not needed |
| `method SaveFile` | DONE | crates/core/src/usage_stats.rs |
| `method UpdateAndSend` | N/A | network upload, PLAN 6 |
| `method TrueSend` | N/A | network upload, PLAN 6 |
| `class UsageStatsRV` | N/A | network upload, PLAN 6 |
| `field UpdateResult` | DONE | crates/core/src/usage_stats.rs |
| `field Messages` | N/A | server news messages, PLAN 6 |
| `property Success` | DONE | crates/core/src/usage_stats.rs |
| `class MessageManager` | N/A | server news messages, PLAN 6 |
| `field Messages` | N/A | server news messages, PLAN 6 |
| `field ClearedIDs` | DONE | crates/core/src/usage_stats.rs |
| `method GetMessages` | DONE | crates/core/src/usage_stats.rs |
| `method ClearMessage` | DONE | crates/core/src/usage_stats.rs |
| `method SetMessages` | DONE | crates/core/src/usage_stats.rs |
| `method GetMessage` | DONE | crates/core/src/usage_stats.rs |
| `class MessageData` | N/A | server news messages, PLAN 6 |
| `enum MessageTypes` | DONE | crates/core/src/usage_stats.rs |
| `field id` | DONE | crates/core/src/usage_stats.rs |
| `field date` | DONE | crates/core/src/usage_stats.rs |
| `field type` | DONE | crates/core/src/usage_stats.rs |
| `field title` | DONE | crates/core/src/usage_stats.rs |
| `field content` | DONE | crates/core/src/usage_stats.rs |
| `field date_from` | DONE | crates/core/src/usage_stats.rs |
| `field date_to` | DONE | crates/core/src/usage_stats.rs |
| `field clearable` | DONE | crates/core/src/usage_stats.rs |
| `property ID` | DONE | crates/core/src/usage_stats.rs |
| `property Date` | DONE | crates/core/src/usage_stats.rs |
| `property Type` | DONE | crates/core/src/usage_stats.rs |
| `property Title` | DONE | crates/core/src/usage_stats.rs |
| `property Content` | DONE | crates/core/src/usage_stats.rs |
| `property DateFrom` | DONE | crates/core/src/usage_stats.rs |
| `property DateTo` | DONE | crates/core/src/usage_stats.rs |
| `property Clearable` | DONE | crates/core/src/usage_stats.rs |
| `property iType` | DONE | crates/core/src/usage_stats.rs |
| `class MyWebClient` | N/A | network upload, PLAN 6 |
| `method GetWebRequest` | DONE | crates/core/src/usage_stats.rs |
| `method GetID` | DONE | crates/core/src/usage_stats.rs |

## `UserControls/ColorProgressBar.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `enum FillStyles` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `class ColorProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _Value` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _Minimum` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _Maximum` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _Step` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _Reverse` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _DrawProgressString` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _ProgressStringDecimals` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _ThrowExceprion` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _FillStyle` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _FillColor` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _BarColor` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field _BorderColor` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `ctor ColorProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method PerformStep` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method PerformStepBack` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method Increment` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method Decrement` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawString` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawBorder` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawBackground` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field L` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `field S` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawProgres` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawBar` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method DrawTick` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |
| `method SetBoundsCore` | DONE | `app/src/ui/widgets/progress.rs` (`ColorProgressBar`) |

## `UserControls/ColorProgressBarDesigner.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ColorProgressBarDesigner` | N/A | WinForms design-time smart tag only |
| `ctor ColorProgressBarDesigner` | N/A | WinForms design-time smart tag only |
| `method PostFilterProperties` | N/A | WinForms design-time smart tag only |

## `UserControls/ColorSlider.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ColorSlider` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field ValueChanged` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field Scroll` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barHalfRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbHalfRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field elapsedRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbSize` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbCustomShape` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbRoundRectSize` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field borderRoundRectSize` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barOrientation` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field trackerValue` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barMinimum` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barMaximum` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field smallChange` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field largeChange` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field drawFocusRectangle` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field drawSemitransparentThumb` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field mouseEffects` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field mouseWheelBarPartitions` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbOuterColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbInnerColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field thumbPenColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barOuterColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barInnerColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field barPenColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field elapsedOuterColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field elapsedInnerColor` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field aColorSchema` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `enum ColorSchemas` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field colorSchema` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `ctor ColorSlider` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `ctor ColorSlider` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method DrawColorSlider` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field mouseInRegion` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnEnabledChanged` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseEnter` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseLeave` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseDown` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `field mouseInThumbRegion` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseMove` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseUp` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnMouseWheel` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnGotFocus` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnLostFocus` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method OnKeyUp` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method ProcessDialogKey` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method CreateRoundRectPath` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method DesaturateColors` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method LightenColors` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method SetProperValue` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method IsPointInRect` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |
| `method RiseChanged` | DONE | `app/src/ui/widgets/color_slider.rs` (`ColorSlider`, `SliderColors`) |

## `UserControls/ComboBoxEnumItem.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class EnumComboBox` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `method AddItem` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `method Clear` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `class ComboBoxEnumItem` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `field mValue` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `field mTrim` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `ctor ComboBoxEnumItem` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `method ToString` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `method Equals` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |
| `method GetHashCode` | DONE | `app/src/ui/widgets/combo.rs` (`enum_combo`, `enum_item_text`) |

## `UserControls/CommandLog.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class CommandLog` | DONE | app/src/forms/main/command_log.rs |
| `ctor CommandLog` | DONE | app/src/forms/main/command_log.rs |
| `method SetCom` | DONE | app/src/forms/main/command_log.rs |
| `method OnPaintBackground` | DONE | app/src/forms/main/command_log.rs |
| `method OnPaint` | DONE | app/src/forms/main/command_log.rs |
| `method TimerUpdate` | DONE | app/src/forms/main/command_log.rs |
| `method OnColorChange` | DONE | app/src/forms/main/command_log.rs |

## `UserControls/DoubleProgressBar.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class DoubleProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `ctor DoubleProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `class Bar` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_value` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_color` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `ctor Bar` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `class Reference` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_value` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_color` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_bcolor` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_bottom` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `ctor Reference` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_Bars` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_References` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `field m_PercString` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `method DrawProgres` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `method DrawRef` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |
| `method DrawString` | DONE | `app/src/ui/widgets/progress.rs` (`DoubleProgressBar`) |

## `UserControls/FlatComboBox.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class FlatComboBox` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method WndProc` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field WM_PAINT` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `struct RECT` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field B` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `struct PAINTSTRUCT` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field hdc` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field fErase` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field rcPaint_left` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field rcPaint_top` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field rcPaint_right` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field rcPaint_bottom` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field fRestore` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field fIncUpdate` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved1` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved2` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved3` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved4` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved5` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved6` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved7` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `field reserved8` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method BeginPaint` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method EndPaint` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method SelectClipRgn` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method GetUpdateRgn` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `enum RegionFlags` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method DeleteObject` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |
| `method CreateRectRgn` | DONE | `app/src/ui/widgets/combo.rs` (`flat_combo`; owner-draw WndProc is WinForms-only) |

## `UserControls/GrblButton.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblButton` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `field mHover` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `field mPressed` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `ctor GrblButton` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `method OnMouseEnter` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `method OnMouseLeave` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `method OnMouseDown` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |
| `method OnMouseUp` | DONE | `app/src/ui/widgets/grbl_button.rs` (`GrblButton`) |

## `UserControls/GrblGroupBox.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblGroupBox` | DONE | `app/src/ui/widgets/group_box.rs` (`grbl_group_box`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/group_box.rs` (`grbl_group_box`) |

## `UserControls/GrblPanel.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblPanel` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `field mLastWPos` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `field mLastMPos` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `field mCurF` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `field mCurS` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `field mFSTrig` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `ctor GrblPanel` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method SettingsForm_SettingsChanged` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method OnPaintBackground` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method OnPaint` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method GetPen` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method GetPen` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method GetBrush` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method SetCore` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method RecreateBMP` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method AbortCreation` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method OnSizeChanged` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method DoTheWork` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method MachineToDraw` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method DrawToMachine` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method AssignBMP` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method TimerUpdate` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method OnColorChange` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |
| `method GrblPanel_MouseDoubleClick` | DONE | `app/src/ui/preview/panel_cpu.rs` (`CpuPanel`, `render`, `rasterize`, `invert`) |

## `UserControls/GrblPanel3D.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblPanel3D` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mLastMousePos` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mMousePos` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mMouseWorldPosition` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mCamera` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mBackgroundColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mTicksColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mMinorsColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mOriginsColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mPointerColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mTextColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mTextBoundingColor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mFontName` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property mTextFont` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mGrid` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mGrbl3D` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mGrbl3DOff` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mGrbl3DLock` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mMessage` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mReload` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mInvalidateAll` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mPadding` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field Core` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `property PointerSize` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mLastControlSize` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mThreadDraw` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `field mBmp` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field OpenGL` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `field mLastWPos` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mLastMPos` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mCurF` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mCurS` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field mFSTrig` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field FrameTime` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field SleepTime` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field RefreshRate` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field FatalException` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `field mShowCursor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field OpCounter` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `field CurrentVendor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field CurrentRenderer` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field CurrentGLVersion` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field FirstGlError` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field RR` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `ctor GrblPanel3D` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GetRulerStep` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method Dispose` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method InitializeOpenGL` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method DrawScene` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method BestSleep` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method SetWorldPosition` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblPanel3D_Resize` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblPanel3D_DoubleClick` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field errcounter` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method CheckError` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method GrblPanel3D_Disposed` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method DrawPointer` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method HumanReadableLength` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method MeasureOpenGlText` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DrawText` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DrawRulers` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method OnPaint` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DrawException` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method FormatCoord` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DoGDIDraw` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field top` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DrawCross` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field halfCrossSize` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GetShortcut` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DrawOverlay` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method MeasureText` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblSceneControl_MouseLeave` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblSceneControl_MouseUp` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblSceneControl_MouseDown` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblSceneControl_MouseMove` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblSceneControl_MouseWheel` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method SetCore` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method OnFileLoading` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method OnProgramEnded` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method Core_OnAutoSizeDrawing` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method Core_OnZoomInDrawing` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method Core_OnZoomOutDrawing` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method PrerviewLineSize_OnChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method ShowLaserOffMovements_OnChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method ShowBoundingBox_OnChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method ShowExecutedCommands_OnChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method DisposeGrbl3D` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method OnFileLoaded` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method AutoSizeDrawing` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field growFactor` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `field defaultViewport` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method TimerUpdate` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method Grbl3D_OnLoadingPercentageChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method OnColorChange` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method GrblPanel3D_Load` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method TimDetectIssue_Tick` | N/A | SharpGL/WinForms specific; GL set up lazily in paint callback, failure falls back to CPU renderer |
| `method ZoomIn` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method ZoomOut` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |
| `method CenterZoom` | DONE | `app/src/ui/preview/panel_gl.rs` (`GlPanel`), `camera.rs`, `gl.rs`, `overlay.rs` |

## `UserControls/GrblTextBox.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class GrblTextBox` | DONE | `app/src/ui/widgets/text.rs` (`GrblTextBox`) |
| `delegate void` | DONE | `app/src/ui/widgets/text.rs` (`GrblTextBox`) |
| `field CommandEntered` | DONE | `app/src/ui/widgets/text.rs` (`GrblTextBox`) |
| `ctor GrblTextBox` | DONE | `app/src/ui/widgets/text.rs` (`GrblTextBox`) |
| `method ProcessCmdKey` | DONE | `app/src/ui/widgets/text.rs` (`GrblTextBox`) |

## `UserControls/IGrblPanel.cs` (WP-U2)

| member | status | Rust location / reason |
|---|---|---|
| `interface IGrblPanel` | DONE | `app/src/ui/preview/mod.rs` (`PreviewPanel::show`, `PreviewInput`, `PreviewEvent`) |

## `UserControls/ImageButton.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ImageButton` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field CAPTION_FONTSIZE` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field CAPTION_HEIGHT` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field CLICK_SCALE_IN_PIXEL` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `ctor ImageButton` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method Init` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field _image` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `property Caption` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field _altimage` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field _UseAltImage` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `enum SizingModes` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `field _sizingmode` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `property RoundedBorders` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method IsMouseInside` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method DrawDisabled` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method ImageButtonMouseEnter` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method ImageButtonMouseLeave` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method ImageButtonMouseUp` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method ImageButtonMouseDown` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method ImageButtonEnabledChanged` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method OnControlAdded` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |
| `method OnControlRemoved` | DONE | `app/src/ui/widgets/image_button.rs` (`ImageButton`) |

## `UserControls/ImageTransform.cs` (WP-U3)

| member | status | Rust location / reason |
|---|---|---|
| `class ImageTransform` | N/A | GDI+ ColorMatrix helpers used only by ImageButton; mirrored by `add_brightness` + tinting in ui/icons.rs |
| `method Negative` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method Brightness` | DONE | app/src/ui/widgets/image_button.rs (`ImageButton`) `add_brightness` |
| `method Coloration` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method Translate` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method Translate` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method MakeWhite` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method SetColor` | DONE | app/src/ui/widgets/image_button.rs (`ImageButton`) disabled tint |
| `method ChangeAlpha` | N/A | static helper with no caller anywhere in the original (grep verified); only reachable from other uncalled helpers |
| `method draw_adjusted_image` | N/A | ColorMatrix plumbing, not needed |
| `enum Formula` | N/A | CCIRRec709 formula of the ImageButton gray look, handled in image_button.rs |
| `method GrayScale` | DONE | app/src/ui/widgets/image_button.rs (`ImageButton`) disabled look |

## `UserControls/LabelTB.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class LabelTB` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `ctor LabelTB` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `method TB_DoubleClick` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `method TB_ValueChanged` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `field _basetext` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `method RefreshText` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `method TB_MouseUp` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |
| `method BtnReset_Click` | DONE | `app/src/ui/widgets/label_tb.rs` (`LabelTb`) + `track_bar.rs` (`TrackBar`) |

## `UserControls/Move2DControl.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class Move2DControl` | N/A | control is not used by any form of the original |
| `class InteractionZone` | N/A | control is not used by any form of the original |
| `ctor InteractionZone` | N/A | control is not used by any form of the original |
| `property Path` | N/A | control is not used by any form of the original |
| `property Color` | N/A | control is not used by any form of the original |
| `method HitTest` | N/A | control is not used by any form of the original |
| `method Draw` | N/A | control is not used by any form of the original |
| `class SpeedPercentZone` | N/A | control is not used by any form of the original |
| `ctor SpeedPercentZone` | N/A | control is not used by any form of the original |
| `field Speed` | N/A | control is not used by any form of the original |
| `class MouvementZone` | N/A | control is not used by any form of the original |
| `ctor MouvementZone` | N/A | control is not used by any form of the original |
| `property Mouvement` | N/A | control is not used by any form of the original |
| `property IsHome` | N/A | control is not used by any form of the original |
| `property IsLeft` | N/A | control is not used by any form of the original |
| `property IsRight` | N/A | control is not used by any form of the original |
| `property IsUp` | N/A | control is not used by any form of the original |
| `property IsDown` | N/A | control is not used by any form of the original |
| `ctor Move2DControl` | N/A | control is not used by any form of the original |
| `field m_SpeedMaximum` | N/A | control is not used by any form of the original |
| `field m_SpeedMinimum` | N/A | control is not used by any form of the original |
| `field m_SpeedValue` | N/A | control is not used by any form of the original |
| `method Move2DControl_Paint` | N/A | control is not used by any form of the original |
| `method PaintMovePart` | N/A | control is not used by any form of the original |
| `method PaintSpeedPart` | N/A | control is not used by any form of the original |
| `method Move2DControl_MouseMove` | N/A | control is not used by any form of the original |
| `method Move2DControl_Click` | N/A | control is not used by any form of the original |
| `method CreateMouvementList` | N/A | control is not used by any form of the original |
| `method CreateMouvementRange` | N/A | control is not used by any form of the original |
| `method CreateMouvement` | N/A | control is not used by any form of the original |
| `delegate void` | N/A | control is not used by any form of the original |
| `class MoveEventArgs` | N/A | control is not used by any form of the original |
| `ctor MoveEventArgs` | N/A | control is not used by any form of the original |
| `field Move` | N/A | control is not used by any form of the original |
| `field F` | N/A | control is not used by any form of the original |
| `field MoveClick` | N/A | control is not used by any form of the original |
| `delegate void` | N/A | control is not used by any form of the original |
| `class HomeEventArgs` | N/A | control is not used by any form of the original |
| `ctor HomeEventArgs` | N/A | control is not used by any form of the original |
| `field F` | N/A | control is not used by any form of the original |
| `field HomeClick` | N/A | control is not used by any form of the original |
| `delegate void` | N/A | control is not used by any form of the original |
| `class SpeedEventArgs` | N/A | control is not used by any form of the original |
| `ctor SpeedEventArgs` | N/A | control is not used by any form of the original |
| `field F` | N/A | control is not used by any form of the original |
| `field SpeedChanged` | N/A | control is not used by any form of the original |
| `method GetPeriferical` | N/A | control is not used by any form of the original |
| `method PaintWithShadow` | N/A | control is not used by any form of the original |
| `method PaintIcon` | N/A | control is not used by any form of the original |
| `method SpeedPercent2Value` | N/A | control is not used by any form of the original |
| `field components` | N/A | control is not used by any form of the original |
| `method Dispose` | N/A | control is not used by any form of the original |
| `method InitializeComponent` | N/A | control is not used by any form of the original |

## `UserControls/MyPictureBox.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class MyPictureBox` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `ctor MyPictureBox` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `field mStage` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `field MouseDownOffsetPt` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `field FenceRect` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method OnMouseDown` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method OnMouseMove` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method OnMouseUp` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method GetMouseOverType` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method GetFenceRect` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `field mImage` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method OnPaintBackground` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method DrawPaper` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method DrawImage` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method CalculateResizeToFit` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `method DrawChess` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |
| `property ControlRectangle` | DONE | `app/src/ui/widgets/picture_box.rs` (`PictureBox`) |

## `UserControls/NumericInput/ColoredBorderControl.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ColoredBorderControl` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `method Dispose` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `field _bordercolor` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `property BorderColor` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `method ResetBorderColor` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `method ShouldSerializeBorderColor` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `field _bordersize` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `property BorderSize` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `method OnPaintBackground` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `ctor ColoredBorderControl` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |
| `property DisplayRectangle` | DONE | `app/src/ui/widgets/text.rs` (`colored_border`) |

## `UserControls/NumericInput/DecimalInputBase.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class DecimalInputBase` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field CurrentValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field OnTheFlyValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field DecimalPositionsChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field BeginEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field EndEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field _CurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field _DecimalPositions` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field _inEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field _NormalBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field OldEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field NewEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `field _enabled` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method Init` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `ctor DecimalInputBase` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnSizeChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method ResetInEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method ShouldSerializeInEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method SetCurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method SetCurrentValueAsUser` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method TBKeyDown` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method TBKeyPress` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method TBWheel` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnMouseWheel` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method AcceptKeys` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method FocusEvent` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnEndEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method ParseEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnValueChanging` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method UpdateStatus` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method GetFormatString` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnEnabledChange` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`, shared `edit_box`) |

## `UserControls/NumericInput/DecimalInputRanged.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class DecimalInputRanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method Dispose` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field MaxValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field MinValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field IsInErrorChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field _ValidationEnabled` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field _MaxValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field _MinValue` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field _inErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method ResetInErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method ShouldSerializeInErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field OldErrorState` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method CheckInError` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method UpdateStatus` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method AcceptKeys` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `field _ForceMinMax` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |
| `method OnValueChanging` | DONE | `app/src/ui/widgets/numeric.rs` (`DecimalInputRanged`) |

## `UserControls/NumericInput/IntegerInputBase.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class IntegerInputBase` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method Dispose` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field TB` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field LB` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field OnTheFlyValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field CurrentValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field BeginEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field EndEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field _CurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field _ForcedText` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field _inEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method Init` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnSizeChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property BorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property BackColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property InEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method ResetInEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method ShouldSerializeInEditBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property CurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method SetCurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method SetCurrentValueAsUser` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnValueChanging` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property InEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method TBKeyDown` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method TBKeyPress` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method AcceptKeys` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method FocusEvent` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field OldEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnBeginEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field NewEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnEndEdit` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method ParseEditValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method UpdateStatus` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method GetFormatString` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `field _enabled` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property Enabled` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `property ForcedText` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnEnabledChange` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `ctor IntegerInputBase` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`, shared `edit_box`) |

## `UserControls/NumericInput/IntegerInputRanged.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class IntegerInputRanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method Dispose` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field MaxValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field MinValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field IsInErrorChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `delegate void` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field _ValidationEnabled` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field _MaxValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field _MinValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field _inErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property ValidationEnabled` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property InErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method ResetInErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method ShouldSerializeInErrorBorderColor` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property MaxValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property MinValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property CurrentValue` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property InError` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field OldErrorState` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method CheckInError` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method UpdateStatus` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method AcceptKeys` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `field _ForceMinMax` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `property ForceMinMax` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |
| `method OnValueChanging` | DONE | `app/src/ui/widgets/numeric.rs` (`IntegerInputRanged`) |

## `UserControls/NumericInput/NumericUpDown.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class NumericUpDown` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `field ValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `ctor NumericUpDown` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `method MNumericUpDown_ValueChanged` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `method BeginInit` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `method EndInit` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/numeric.rs` (`NumericUpDown`) |

## `UserControls/PlaceholderTextbox.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class PlaceholderTextBox` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field _waterMarkText` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field _waterMarkColor` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field _waterMarkActiveColor` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field waterMarkContainer` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field waterMarkFont` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `field waterMarkBrush` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `ctor PlaceholderTextBox` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method Initialize` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method RemoveWaterMark` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method DrawWaterMark` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method waterMarkContainer_Click` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method waterMarkContainer_Paint` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method ThisHasFocus` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method ThisWasLeaved` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method ThisTextChanged` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |
| `method OnInvalidated` | DONE | `app/src/ui/widgets/text.rs` (`PlaceholderTextBox`) |

## `UserControls/TextInput.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class TextInput` | DONE | `app/src/ui/widgets/text.rs` (`TextInput`) |
| `ctor TextInput` | DONE | `app/src/ui/widgets/text.rs` (`TextInput`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/text.rs` (`TextInput`) |
| `method mTextBox_TextChanged` | DONE | `app/src/ui/widgets/text.rs` (`TextInput`) |

## `UserControls/ThemeMgr.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class ThemeMgr` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `enum DWMWINDOWATTRIBUTE` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method LoadLibrary` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method GetProcAddress` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method FreeLibrary` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `delegate int` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `field SetWindowTheme` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `delegate int` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `field DwmSetWindowAttribute` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method Initialize` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method GetDelegate` | N/A | Win32 DWM/UxTheme interop; egui paints its own chrome via `Theme::apply` |
| `method SetTheme` | DONE | `ui/theme.rs` (`Theme::apply`: dark/light chrome + colors) |
| `method SetTheme` | DONE | `ui/theme.rs` (`Theme::apply`: dark/light chrome + colors) |

## `UserControls/UsageClassPreview.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class UsageClassPreview` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `field mLLC` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `field Rectangles` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `field mCol` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `ctor UsageClassPreview` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `method OnPaint` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `method OnMouseMove` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |
| `method ComputeRect` | DONE | `app/src/ui/widgets/usage_class_preview.rs` (`UsageClassPreview`) |

## `UserControls/WaitingProgressBar.cs` (WP-U1)

| member | status | Rust location / reason |
|---|---|---|
| `class WaitingProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `ctor WaitingProgressBar` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method Dispose` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `field TIM` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method InitializeComponent` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `property Running` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `property Interval` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method Start` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method Stop` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method TIM_Elapsed` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `enum BouncingModeEnum` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `field _BouncingMode` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `property BouncingMode` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |
| `method DrawProgres` | DONE | `app/src/ui/widgets/progress.rs` (`WaitingProgressBar`) |

## `WiFiConfigurator/OrturWiFiConfig.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class OrturWiFiConfig` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `field Core` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `field mWaitMsg` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `field mWaitTime` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `ctor OrturWiFiConfig` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method FindPass` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method FindSSID` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method PbRevealPwd_MouseDown` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method PbRevealPwd_MouseUp` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method OnTextChanges` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method BtnWrite_Click` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method WaitMode` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method OrturWiFiConfig_FormClosing` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method TimWait_Tick` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |
| `method BtnHelp_Click` | DONE | app/src/forms/dialogs/ortur_wifi_config.rs |

## `WiFiConfigurator/OrturWiFiConfigPrompt.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class WiFiConfigPrompt` | DONE | app/src/forms/dialogs/ortur_wifi_config_prompt.rs |
| `ctor WiFiConfigPrompt` | DONE | app/src/forms/dialogs/ortur_wifi_config_prompt.rs |
| `method CbDontShow_CheckedChanged` | DONE | app/src/forms/dialogs/ortur_wifi_config_prompt.rs |

## `WiFiDiscovery/DiscoveryForm.cs` (WP-U4)

| member | status | Rust location / reason |
|---|---|---|
| `class DiscoveryForm` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `ctor DiscoveryForm` | DONE | crates/services/src/wifi_discovery.rs default_port |
| `method BtnScan_Click` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method StartScan` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method ScanAsync` | DONE | crates/services/src/wifi_discovery.rs scan_ip |
| `method BtnStop_Click` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method StopScan` | DONE | crates/services/src/cancel flag AtomicBool passed to scan_ip |
| `method CreateAndShowDialog` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method LV_SelectedIndexChanged` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method BtnConnect_Click` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method ReturnItem` | DONE | crates/services/src/wifi_discovery.rs result_address |
| `method DiscoveryForm_FormClosing` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method UdPort_ValueChanged` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method LV_MouseDoubleClick` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `class ResultItem` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `field RI` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `ctor ResultItem` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method OnUpdate` | DONE | app/src/forms/dialogs/discovery_form.rs |
| `method DoRefresh` | DONE | app/src/forms/dialogs/discovery_form.rs |

## `WiFiDiscovery/IPAddressHelper.cs` (WP-C2)

| member | status | Rust location / reason |
|---|---|---|
| `class IPAddressHelper` | DONE | crates/services/src/wifi_discovery.rs |
| `field pbuff` | N/A | ping payload fixed in ping() |
| `field AVAIL` | DONE | crates/services/src/wifi_discovery.rs |
| `method SendARP` | DONE | crates/services/src/wifi_discovery.rs get_mac (ARP cache instead of iphlpapi) |
| `method GetBroadcastAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `method GetNetworkAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `method GetSubnetMask` | DONE | crates/services/src/wifi_discovery.rs |
| `method GetLocalIP` | DONE | crates/services/src/wifi_discovery.rs |
| `class ScanResult` | DONE | crates/services/src/wifi_discovery.rs |
| `field Update` | DONE | crates/services/src/wifi_discovery.rs |
| `field notified` | DONE | crates/services/src/wifi_discovery.rs |
| `field mPingTime` | DONE | crates/services/src/wifi_discovery.rs |
| `field MAC` | DONE | crates/services/src/wifi_discovery.rs |
| `field HostName` | DONE | crates/services/src/wifi_discovery.rs |
| `field Telnet` | DONE | crates/services/src/wifi_discovery.rs |
| `field IP` | DONE | crates/services/src/wifi_discovery.rs |
| `field Port` | DONE | crates/services/src/wifi_discovery.rs |
| `ctor ScanResult` | DONE | crates/services/src/wifi_discovery.rs |
| `ctor ScanResult` | DONE | crates/services/src/wifi_discovery.rs |
| `method HasData` | DONE | crates/services/src/wifi_discovery.rs |
| `method DeepScan` | DONE | crates/services/src/wifi_discovery.rs |
| `method SetPing` | DONE | crates/services/src/wifi_discovery.rs |
| `property Ping` | DONE | crates/services/src/wifi_discovery.rs |
| `method ToString` | DONE | crates/services/src/wifi_discovery.rs |
| `method GetMAC` | DONE | crates/services/src/wifi_discovery.rs |
| `method GetHostName` | DONE | crates/services/src/wifi_discovery.rs |
| `method Telnet` | DONE | crates/services/src/wifi_discovery.rs |
| `method Ping` | DONE | crates/services/src/wifi_discovery.rs |
| `method IsInSameSubnet` | DONE | crates/services/src/wifi_discovery.rs |
| `method ToUInt32` | DONE | crates/services/src/wifi_discovery.rs |
| `method ScanIP` | DONE | crates/services/src/wifi_discovery.rs |
| `class UInt16Helper` | DONE | crates/services/src/wifi_discovery.rs |
| `method ReverseBytes` | DONE | crates/services/src/wifi_discovery.rs |
| `class UInt32Helper` | DONE | crates/services/src/wifi_discovery.rs |
| `method ToIPAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `method ReverseBytes` | DONE | crates/services/src/wifi_discovery.rs |
| `class UInt64Helper` | DONE | crates/services/src/wifi_discovery.rs |
| `method ReverseBytes` | DONE | crates/services/src/wifi_discovery.rs |
| `field mRespCount` | DONE | crates/services/src/wifi_discovery.rs build_responding_list |
| `field mPort` | DONE | crates/services/src/wifi_discovery.rs |
| `ctor IPSegmentScanner` | DONE | crates/services/src/wifi_discovery.rs |
| `method FullScan` | DONE | crates/services/src/wifi_discovery.rs |
| `method FastScan` | DONE | crates/services/src/wifi_discovery.rs |
| `method BuildFullList` | DONE | crates/services/src/wifi_discovery.rs |
| `method BuildRespondingList` | DONE | crates/services/src/wifi_discovery.rs |
| `method PingCompleted` | DONE | crates/services/src/wifi_discovery.rs build_responding_list (worker pool) |
| `method Analize` | DONE | crates/services/src/wifi_discovery.rs |
| `field ip` | DONE | crates/services/src/wifi_discovery.rs |
| `field mask` | DONE | crates/services/src/wifi_discovery.rs |
| `field networkAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `field broadcastAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `field mAllHost` | DONE | crates/services/src/wifi_discovery.rs |
| `ctor IPSegment` | DONE | crates/services/src/wifi_discovery.rs |
| `property NetworkAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `property BroadcastAddress` | DONE | crates/services/src/wifi_discovery.rs |
| `property HostToScan` | DONE | crates/services/src/wifi_discovery.rs |
