package ping

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"google.golang.org/protobuf/types/known/emptypb"
)

func TestPing(t *testing.T) {
	ds := &Server{}
	res, err := ds.Ping(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)
	assert.Equal(t, res.Component, "sysd")
}
