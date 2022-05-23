import CHedera
import Foundation

/// A transaction or query that can be executed on the Hedera network.
public class Request<Response: Decodable>: Encodable {
    internal init() {}

    /// Execute this request against the provided client of the Hedera network.
    public func execute(_ client: Client) async throws -> Response {
        // encode self as a JSON request to pass to Rust
        let requestBytes = try JSONEncoder().encode(self)
        let request = String(data: requestBytes, encoding: .utf8)!

        // start an unmanaged continuation to bridge a C callback with Swift async
        let responseBytes: Data = try await withUnmanagedThrowingContinuation { continuation in
            // invoke `hedera_execute`, callback will be invoked on request completion
            hedera_execute(client.ptr, request, continuation) { continuation, err, responsePtr in
                if err != HEDERA_ERROR_OK {
                    // an error has occurred, consume from the TLS storage for the error
                    // and throw it up back to the async task
                    resumeUnmanagedContinuation(continuation, throwing: HError(err)!)
                } else {
                    // NOTE: we are guaranteed to receive valid UTF-8 on a successful response
                    let responseBytes = String(validatingUTF8: responsePtr!)!.data(using: .utf8)!

                    // resumes the continuation which bridges us back into Swift async
                    resumeUnmanagedContinuation(continuation, returning: responseBytes)
                }
            }
        }

        // decode the response as the generic output type of this query types
        let response = try JSONDecoder().decode(Response.self, from: responseBytes)

        return response
    }

    public func encode(to encoder: Encoder) throws {
        // nothing to encode at this level
    }
}
