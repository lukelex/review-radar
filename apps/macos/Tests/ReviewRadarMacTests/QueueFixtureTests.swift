import XCTest
@testable import ReviewRadarMac

final class QueueFixtureTests: XCTestCase {
    func testFixtureDecodesTheVersionedQueueResponse() throws {
        let url = try XCTUnwrap(Bundle.module.url(forResource: "queue-response", withExtension: "json"))
        let response = try JSONDecoder().decode(QueueResponse.self, from: Data(contentsOf: url))
        XCTAssertEqual(response.schemaVersion, 1)
        XCTAssertEqual(response.view, "tailored")
        XCTAssertEqual(response.ranking, "tailored")
        XCTAssertEqual(response.count, 1)
        XCTAssertEqual(response.pullRequests.first?.explanation.reasons.first?.nextAction.label, "Start review")
    }
}
