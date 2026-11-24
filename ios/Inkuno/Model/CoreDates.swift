import Foundation
import InkunoCore

/// Calendar values in the Rust core's terms: the core takes days and times, not `Date`.
enum CoreDates {
    static func ymd(_ date: Date, calendar: Calendar = .current) -> InkunoCore.Ymd {
        let parts = calendar.dateComponents([.year, .month, .day], from: date)
        return InkunoCore.Ymd(
            year: Int32(parts.year ?? 0),
            month: UInt32(parts.month ?? 0),
            day: UInt32(parts.day ?? 0)
        )
    }

    static func hms(_ date: Date, calendar: Calendar = .current) -> InkunoCore.Hms {
        let parts = calendar.dateComponents([.hour, .minute, .second], from: date)
        return InkunoCore.Hms(
            hour: UInt32(parts.hour ?? 0),
            minute: UInt32(parts.minute ?? 0),
            second: UInt32(parts.second ?? 0)
        )
    }
}
